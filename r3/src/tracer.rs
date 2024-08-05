use log::{debug, info, warn};
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::{LazyLock, Mutex};
use serde::{Serialize, Deserialize};
use postcard;
use wamr_rust_sdk::wasm_exec_env_t;
use libc::gettid;

pub static GLOBAL_TRACE: LazyLock<Mutex<Vec<TraceOp>>> = LazyLock::new(|| Mutex::new(vec![]));

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum CallID {
    ScUnknown,
    ScMmap { grow: i32 },
    ScWritev { fd: i32, iov: i32, iovcnt: i32 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Access {
    access_idx: i32,
    opcode: i32,
    addr: i32,
    size: i32,
    load_value: i64,
    expected_value: i64,
}
impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Access [{} | {:#04X}] @ Addr [{:6}::{}] with Read [{:#0vwidth$X}] ==/== [{:#0vwidth$X}]", 
            self.access_idx, self.opcode, self.addr, self.size, self.load_value, self.expected_value, vwidth = (self.size as usize * 2)+2)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Call {
    access_idx: i32,
    opcode: i32,
    func_idx: i32,
    call_id: CallID,
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

/* Convert engine-level CallID to Rust Enum */
fn create_call_id(call_id: i32, args: (i32, i32, i32)) -> Option<CallID> {
    match call_id {
        0 => Some(CallID::ScUnknown),
        1 => Some(CallID::ScMmap { grow: args.0 }),
        2 => Some(CallID::ScWritev { fd: args.0, iov: args.1, iovcnt: args.2 }),
        _ => None,
    }
}

/* Record Op to global trace */
fn add_to_global_trace(op: TraceOp) {
    let mut trace = GLOBAL_TRACE.lock().unwrap();
    trace.push(op);
}

pub fn dump_global_trace(tracefile: &String) -> io::Result<()>{
    let mut file = File::create(tracefile)?;
    let trace = GLOBAL_TRACE.lock().unwrap();
    let ser: Vec<u8> = postcard::to_stdvec(&*trace).unwrap();
    file.write_all(&ser)?;

    /* Verify serialization can be effectively deserialized */
    let deserialized: Vec<TraceOp> = postcard::from_bytes(&ser).unwrap();
    assert_eq!(*trace, deserialized);
    Ok(())
}

/* Wasm Engine Hook: Records MemOps */
pub extern "C" fn wasm_memop_tracedump(_exec_env: wasm_exec_env_t, differ: i32, access_idx: i32, opcode: i32, addr: i32, size: i32, load_value: i64, expected_value: i64) {
    if addr == 0 {
        warn!("[{} | {:#04X}] Access to address [{}::{}] may be invalid", access_idx, opcode, addr, size);
    }
    if differ != 0 {
        let tidval = unsafe { gettid() };
        let access = Access { access_idx, opcode, addr, size, load_value, expected_value };
        debug!("[{}] [Trace MEMOP] {} | Diff? {}", tidval, access, differ);
        /* Add to trace here */
        add_to_global_trace(TraceOp::MemOp(access));
    }
}

/* Wasm Engine Hook: Records CallOps */
pub extern "C" fn wasm_call_tracedump(_exec_env: wasm_exec_env_t, access_idx: i32, opcode: i32, func_idx: i32, 
        call_id: i32, a1: i64, a2: i64, a3: i64) {
    if opcode != 0x10 {
        warn!("[{} | {:#04X}] Unexpected opcode", access_idx, opcode);
    }
    let tidval = unsafe { gettid() };
    let call_id = create_call_id(call_id, (a1 as i32, a2 as i32, a3 as i32)).unwrap();
    let call_trace = Call { access_idx, opcode, func_idx, call_id };
    debug!("[{}] [Trace CALL] {}", tidval, call_trace); 
    /* Add to trace here */
    add_to_global_trace(TraceOp::CallOp(call_trace));
}