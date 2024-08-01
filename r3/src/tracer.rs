use log::{debug, info, warn};
use wamr_rust_sdk::wasm_exec_env_t;
use libc::gettid;

pub extern "C" fn wasm_memop_tracedump(_exec_env: wasm_exec_env_t, differ: i32, access_idx: i32, opcode: i32, addr: i32, size: i32, load_value: i64, expected_value: i64) {
    if addr == 0 {
        warn!("[{} | {:#04X}] Access to address 0 is likely invalid", access_idx, opcode);
    }
    if differ != 0 {
        let tidval = unsafe { gettid() };
        debug!("[{}] [Trace MEMOP] [{:6} | {:#6X}] @ Addr [{:8}::{}] with Value [{:#016X}] (Expected [{:#016X}]), Diff? {}", 
            tidval, access_idx, opcode, addr, size, load_value, expected_value, differ);
    }
}

pub extern "C" fn wasm_call_tracedump(_exec_env: wasm_exec_env_t, access_idx: i32, opcode: i32, func_idx: i32) {
    if opcode != 0x10 {
        warn!("[{} | {:#04X}] Unseen opcode", access_idx, opcode);
    }
    let tidval = unsafe { gettid() };
    debug!("[{}] [Trace CALL] [{:6} | {:#6X}] @ FuncIdx [{:6}]", 
        tidval, access_idx, opcode, func_idx);
}