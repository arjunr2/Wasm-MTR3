//! Utilities to implement foreign function interface for running replay
//! modules -- This is the **replay interface** for any new Wasm engine to run
//! modules.
use libc;
use log::{debug, trace, warn};
use std::process;

use common::trace::{CallID, ReplayPropLogInfo};
use common::wasm2native::*;
use wamr_rust_sdk::{wasm_cluster_cancel_thread, wasm_exec_env_t};

/// Exit a process immediately
pub fn wasm_r3_replay_proc_exit(_exec_env: wasm_exec_env_t, code: i32) {
    debug!("ProcExit | Exiting process with code: {}", code);
    process::exit(code);
}

/// Exit a thread immediately
pub fn wasm_r3_replay_thread_exit(exec_env: wasm_exec_env_t, code: i32) {
    debug!("ThreadExit | Exiting thread with code: {}", code);
    unsafe {
        wasm_cluster_cancel_thread(exec_env);
    }
}

/// (`debug`) Perform a [`writev`] operation
///
/// [`writev`]: https://linux.die.net/man/2/writev
pub fn wasm_r3_replay_writev(
    exec_env: wasm_exec_env_t,
    fd: i32,
    iovs: WasmAddr,
    iovcnt: i32,
) -> i64 {
    debug!("Writev | fd: {}, iovs: {}, iovcnt: {} ", fd, iovs, iovcnt);
    let native_iovs = unsafe { get_native_iovec_from_wali(exec_env, iovs, iovcnt) };
    unsafe {
        if fd != 1 {
            warn!(
                "Writev | Only fd=1 (stdout) supported for debug; got {}",
                fd
            );
            0
        } else {
            libc::writev(
                fd,
                native_iovs.as_ptr() as *const libc::iovec,
                iovcnt as i32,
            ) as i64
        }
    }
}

/// (`debug`) Log a `futex` synchronization operations with
/// its arguments
pub fn wasm_r3_replay_futex_log(_exec_env: wasm_exec_env_t, addr: i32, op: i32, val: i32) {
    debug!(
        "Futex Log | {:?}[{}], val: {}",
        FutexOp::from_i32(op),
        addr,
        val
    );
}

/// (`debug`) Log a single dynamic replay operation with
/// its properties
pub fn wasm_r3_replay_log_call(
    _exec_env: wasm_exec_env_t,
    access_idx: u32,
    func_idx: u32,
    tid: u32,
    prop_idx: u32,
    call_id: u32,
    return_val: i64,
    a1: i64,
    a2: i64,
    a3: i64,
    sync_id: u64,
) {
    let call_id = CallID::from_parts(call_id, [a1, a2, a3]).unwrap();
    debug!(
        "{}",
        ReplayPropLogInfo {
            access_idx,
            func_idx,
            tid: tid as u64,
            prop_idx,
            call_id,
            return_val,
            sync_id
        }
    );
}

/// Get the current thread ID
///
/// TIDs start with 0 and sequentially increment in order of creation
/// TID 0 should be used for start function and TID 1 for main function
///   and sequentially increment in order of creation
pub fn wasm_r3_replay_gettid(exec_env: wasm_exec_env_t) -> u32 {
    let tid = get_wasmtid(exec_env);
    trace!("GetTID | {}", tid);
    tid as u32
}
