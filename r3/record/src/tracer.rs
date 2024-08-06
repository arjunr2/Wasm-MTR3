use log::{debug, warn};
use std::io::{self, Write};
use std::fs::File;
use std::sync::{LazyLock, Mutex};
use wamr_rust_sdk::wasm_exec_env_t;
use libc::gettid;

use common::trace::*;

pub static GLOBAL_TRACE: LazyLock<Mutex<Vec<TraceOp>>> = LazyLock::new(|| Mutex::new(vec![]));

/* Record Op to global trace */
fn add_to_global_trace(op: TraceOp) {
    let mut trace = GLOBAL_TRACE.lock().unwrap();
    trace.push(op);
}

pub fn dump_global_trace(tracefile: &String, sha256: &str) -> io::Result<()>{
    let mut file = File::create(tracefile)?;
    let trace = GLOBAL_TRACE.lock().unwrap();
    let trace_data = TraceDataSer {
        sha256: sha256,
        trace: &*trace,
    };
    let ser = trace_data.serialize();
    file.write_all(&ser)?;

    /* Verify serialization can be effectively deserialized */
    let deserialized = TraceDataDeser::deserialize(&ser, None);
    assert_eq!(*trace, deserialized.trace);
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