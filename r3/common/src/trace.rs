use std::fmt;
use postcard;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum CallID {
    ScUnknown,
    ScMmap { grow: u32 },
    ScWritev { fd: i32, iov: i32, iovcnt: u32 },
    ScThreadSpawn,
    ScFutex,
    ScThreadExit { status: i32 },
    ScProcExit { status: i32 },
    ScGeneric,
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
        write!(f, "Access [{} | {:#04X}] @ Addr [{:6}::{}] with Read [{:#0vwidth$X}] ==/== [{:#0vwidth$X}]", 
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
        write!(f, "Call [{:6} | {:#6X}] for [{:?} | {:3}] with Return [{:#X}]", 
            self.access_idx, self.opcode, self.call_id, self.func_idx, self.return_val)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TraceOp {
    MemOp(Access),
    CallOp(Call)
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

/* Convert instrumentation-level CallID to Rust Enum */
pub fn create_call_id(call_id: u32, args: [i64;3]) -> Option<CallID> {
    match call_id {
        0 => Some(CallID::ScUnknown),
        1 => Some(CallID::ScMmap { grow: args[0] as u32 }),
        2 => Some(CallID::ScWritev { fd: args[0] as i32, iov: args[1] as i32, iovcnt: args[2] as u32 }),
        3 => Some(CallID::ScThreadSpawn),
        4 => Some(CallID::ScFutex),
        5 => Some(CallID::ScThreadExit { status: args[0] as i32 }),
        6 => Some(CallID::ScProcExit { status: args[0] as i32 }),
        0xFFFFFFFF => Some(CallID::ScGeneric),
        _ => None,
    }
}



/* Convert Rust Enum CallID to instrumentation-level call_id */
pub fn get_instrumentation_call_id(call_id: &CallID) -> (u32, [i64;3]) {
    match call_id {
        CallID::ScUnknown => (0, [0, 0, 0]),
        CallID::ScMmap { grow } => (1, [*grow as i64, 0, 0]),
        CallID::ScWritev { fd, iov, iovcnt } => (2, [*fd as i64, *iov as i64, *iovcnt as i64]),
        CallID::ScThreadSpawn => (3, [0, 0, 0]),
        CallID::ScFutex => (4, [0, 0, 0]),
        CallID::ScThreadExit { status } => (5, [*status as i64, 0, 0]),
        CallID::ScProcExit { status } => (6, [*status as i64, 0, 0]),
        CallID::ScGeneric => (0xFFFFFFFF, [0, 0, 0]),
    }
}
