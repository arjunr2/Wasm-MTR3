use std::fmt;
use postcard;
use serde::{Serialize, Deserialize};
use crate::wasm2native::FutexOp;

/* Enum for import call personality */
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum CallID {
    ScUnknown,
    ScMmap { grow: u32 },
    ScWritev { fd: i32, iov: i32, iovcnt: u32 },
    ScThreadSpawn { fn_ptr: i32, args_ptr: i32 },
    ScFutex { addr: i32, op: FutexOp, val: u32 },
    ScThreadExit { status: i32 },
    ScProcExit { status: i32 },
    ScGeneric,
}
impl CallID {
    /* Convert instrumentation-level CallID (from C) to Rust Enum variant */
    pub fn from_parts(call_id: u32, args: [i64;3]) -> Option<Self> {
        match call_id {
            0 => Some(CallID::ScUnknown),
            1 => Some(CallID::ScMmap { grow: args[0] as u32 }),
            2 => Some(CallID::ScWritev { fd: args[0] as i32, iov: args[1] as i32, iovcnt: args[2] as u32 }),
            3 => Some(CallID::ScThreadSpawn { fn_ptr: args[0] as i32, args_ptr: args[1] as i32 }),
            4 => Some(CallID::ScFutex { addr: args[0] as i32, 
                op: FutexOp::from_i32(args[1] as i32), val: args[2] as u32 }),
            5 => Some(CallID::ScThreadExit { status: args[0] as i32 }),
            6 => Some(CallID::ScProcExit { status: args[0] as i32 }),
            0xFFFFFFFF => Some(CallID::ScGeneric),
            _ => None,
        }
    }

    // Reverse conversion of from_parts
    pub fn to_parts(&self) -> (u32, [i64;3]) {
        match self {
            CallID::ScUnknown => (0, [0, 0, 0]),
            CallID::ScMmap { grow } => (1, [*grow as i64, 0, 0]),
            CallID::ScWritev { fd, iov, iovcnt } => (2, [*fd as i64, *iov as i64, *iovcnt as i64]),
            CallID::ScThreadSpawn { fn_ptr, args_ptr } => (3, [*fn_ptr as i64, *args_ptr as i64, 0]),
            CallID::ScFutex { addr, op, val } => (4, [*addr as i64, *op as i64, *val as i64]),
            CallID::ScThreadExit { status } => (5, [*status as i64, 0, 0]),
            CallID::ScProcExit { status } => (6, [*status as i64, 0, 0]),
            CallID::ScGeneric => (0xFFFFFFFF, [0, 0, 0]),
        }
    }
}



#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TraceOp {
    Access { tid: u64, access_idx: u32, opcode: i32, addr: i32, size: u32, load_value: i64, expected_value: i64, differ: bool },
    Call { tid: u64, access_idx: u32, opcode: i32, func_idx: u32, return_val: i64, call_id: CallID },
    ContextSwitch { access_idx: u32, src_tid: i32, dst_tid: i32 },
}
impl fmt::Display for TraceOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TraceOp::Access { tid, access_idx, opcode, addr, size, load_value, expected_value, differ } => {
                write!(f, "{:>10} [{:>6}::{:>6} | {:#04X}] for Addr [{:6}::{}] with Read [{:#0vwidth$X}] ==/== [{:#0vwidth$X}]", 
                    if *differ { "Access" } else { "UCAccess" },
                    tid, access_idx, opcode, addr, size, load_value, expected_value, vwidth = (*size as usize * 2)+2)
            }
            TraceOp::Call { tid, access_idx, opcode, func_idx, return_val, call_id } => {
                write!(f, "{:>10} [{:>6}::{:>6} | {:#04X}] for [{:?} | {:3}] with Return [{:#X}]", 
                    "Call",
                    tid, access_idx, opcode, call_id, func_idx, return_val)
            }
            TraceOp::ContextSwitch { access_idx, src_tid, dst_tid } => {
                write!(f, "{:>10} [{:>6} | {:7} --> {:7}]", 
                    "CSwitch", 
                    access_idx, src_tid, dst_tid)
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TraceData<'a> {
    pub sha256: &'a str,
    pub trace: Vec<TraceOp>,
}
impl<'a> TraceData<'a> {
    pub fn deserialize(ser: &'a Vec<u8>, sha256: Option<&str>) -> Self {
        let deser: Self = postcard::from_bytes(ser).unwrap();
        if let Some(digest) = sha256 {
            assert_eq!(digest, deser.sha256, "SHA256 mismatch between trace and expected");
        }
        deser
    }
    pub fn serialize(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).unwrap()
    }
}


/// Container for logging all relevant operations for a single replay prop
/// Useful when debugging instrumentation and replay interleaving soundness 
#[derive(Debug)]
pub struct ReplayPropLogInfo {
    pub access_idx: u32,
    pub func_idx: u32,
    pub tid: u64,
    pub prop_idx: u32,
    pub call_id: CallID,
    pub return_val: i64,
    pub sync_id: u64,
}
impl ReplayPropLogInfo {
    pub fn debug_string_header() -> String {
        format!("[{:>8}] -- [{:>3}|{:>8}/{:>6}] | [({:>5}) {} = {:>18}]", 
            "Sync ID", 
            "TID", "Acc#", "Prop#", 
            "Func#", "CallID", "Return Value")
    }
    pub fn to_debug_string(&self) -> String {
        format!("[{:>8}] -- [{:>3}|{:>8}/{:>6}] | [({:>5}) {:?} = {:#16X}]", 
            self.sync_id, 
            self.tid, self.access_idx, self.prop_idx, 
            self.func_idx, self.call_id, self.return_val)
    }
}
impl fmt::Display for ReplayPropLogInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_debug_string())
    }
}

// Order by sync-id for debug
impl Ord for ReplayPropLogInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.sync_id.cmp(&self.sync_id)
    }
}
impl PartialOrd for ReplayPropLogInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for ReplayPropLogInfo {
    fn eq(&self, other: &Self) -> bool {
        self.sync_id == other.sync_id
    }
}
impl Eq for ReplayPropLogInfo {}