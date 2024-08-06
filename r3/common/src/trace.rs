use std::fmt;
use postcard;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum CallID {
    ScUnknown,
    ScMmap { grow: i32 },
    ScWritev { fd: i32, iov: i32, iovcnt: i32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Access {
    pub access_idx: i32,
    pub opcode: i32,
    pub addr: i32,
    pub size: i32,
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
    pub access_idx: i32,
    pub opcode: i32,
    pub func_idx: i32,
    pub call_id: CallID,
}
impl fmt::Display for Call {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Call [{:6} | {:#6X}] for [{:?} | {:3}]", 
            self.access_idx, self.opcode, self.call_id, self.func_idx)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TraceOp {
    MemOp(Access),
    CallOp(Call)
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TraceDataSer<'a> {
    pub sha256: &'a str,
    pub trace: &'a Vec<TraceOp>,
}

impl TraceDataSer<'_> {
    pub fn serialize(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).unwrap()
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct TraceDataDeser<'a> {
    pub sha256: &'a str,
    pub trace: Vec<TraceOp>,
}

impl<'a> TraceDataDeser<'a> {
    pub fn deserialize(ser: &'a Vec<u8>, sha256: Option<&str>) -> Self {
        let deser: Self = postcard::from_bytes(ser).unwrap();
        if let Some(digest) = sha256 {
            assert_eq!(digest, deser.sha256, "SHA256 mismatch between trace and expected");
        }
        deser
    }
}

/* Convert instrumentation-level CallID to Rust Enum */
pub fn create_call_id(call_id: i32, args: (i32, i32, i32)) -> Option<CallID> {
    match call_id {
        0 => Some(CallID::ScUnknown),
        1 => Some(CallID::ScMmap { grow: args.0 }),
        2 => Some(CallID::ScWritev { fd: args.0, iov: args.1, iovcnt: args.2 }),
        _ => None,
    }
}

/* Convert Rust Enum CallID to instrumentation-level call_id */
#[allow(dead_code)]
fn get_instrumentation_call_id(call_id: &CallID) -> (i32, (i32, i32, i32)) {
    match call_id {
        CallID::ScUnknown => (0, (0, 0, 0)),
        CallID::ScMmap { grow } => (1, (*grow, 0, 0)),
        CallID::ScWritev { fd, iov, iovcnt } => (2, (*fd, *iov, *iovcnt)),
    }
}
