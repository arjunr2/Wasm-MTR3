use log::{debug, warn, trace};
use libc;
use std::process;

use wamr_rust_sdk::wasm_exec_env_t;
use common::wasm2native::*;

pub fn wasm_r3_replay_proc_exit(_exec_env: wasm_exec_env_t, code: i32) {
    debug!("ProcExit | Exiting process with code: {}", code);
    process::exit(code);
}

pub fn wasm_r3_replay_thread_exit(_exec_env: wasm_exec_env_t, code: i32) {
    debug!("ThreadExit | Exiting thread with code: {}", code);
    unsafe {
        libc::syscall(libc::SYS_exit, code);
    }
}

pub fn wasm_r3_replay_writev(exec_env: wasm_exec_env_t, fd: i32, iovs: WasmAddr, iovcnt: i32) -> i64 {
    debug!("Writev | fd: {}, iovs: {}, iovcnt: {} ", fd, iovs, iovcnt);
    let native_iovs = unsafe { get_native_iovec_from_wali(exec_env, iovs, iovcnt) };
    unsafe {
        if fd != 1 {
            warn!("Writev | Only fd=1 (stdout) supported for debug; got {}", fd);
            0
        } else {
            libc::writev(fd, 
                native_iovs.as_ptr() as *const libc::iovec, 
                iovcnt as i32) as i64
        }
    }
}

pub fn wasm_r3_replay_futex_log(_exec_env: wasm_exec_env_t, addr: i32, op: i32, val: i32) {
    debug!("Futex Log | {:?}[{}], val: {}", FutexOp::from_i32(op), addr, val);
}

pub fn wasm_r3_replay_gettid(exec_env: wasm_exec_env_t) -> u32 {
    let tid = get_wasmtid(exec_env);
    trace!("GetTID | {}", tid);
    tid as u32
}