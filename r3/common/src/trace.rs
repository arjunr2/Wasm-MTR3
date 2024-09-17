use std::fmt;
use postcard;
use serde::{Serialize, Deserialize};

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


/* Futex Flags */
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum FutexOp {
    Wait = 0,
    Wake = 1,
    Unknown = -1
}
impl FutexOp {
    fn from_i32(op: i32) -> Self {
        // Mask out FUTEX_PRIVATE (bit 7)
        match op & 0x7f {
            0 => FutexOp::Wait,
            1 => FutexOp::Wake,
            _ => FutexOp::Unknown,
        }
    }
}


#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Access {
    pub access_idx: u32,
    pub opcode: i32,
    pub addr: i32,
    pub size: u32,
    pub load_value: i64,
    pub expected_value: i64,
}
impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:>7} [{:>6} | {:#04X}] for Addr [{:6}::{}] with Read [{:#0vwidth$X}] ==/== [{:#0vwidth$X}]", 
            "Access",
            self.access_idx, self.opcode, self.addr, self.size, self.load_value, self.expected_value, vwidth = (self.size as usize * 2)+2)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Call {
    pub access_idx: u32,
    pub opcode: i32,
    pub func_idx: u32,
    pub return_val: i64,
    pub call_id: CallID,
}
impl fmt::Display for Call {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:>7} [{:>6} | {:#04X}] for [{:?} | {:3}] with Return [{:#X}]", 
            "Call",
            self.access_idx, self.opcode, self.call_id, self.func_idx, self.return_val)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ContextSwitch {
    pub tidval: i32,
    pub old_tidval: i32,
}
impl fmt::Display for ContextSwitch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:>7} [{:7} --> {:7}]", "CSwitch", self.old_tidval, self.tidval)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TraceOp {
    MemOp(Access),
    CallOp(Call),
    ContextSwitchOp(ContextSwitch)
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


