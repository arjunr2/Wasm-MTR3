use log::{debug, info, warn};
use wamr_rust_sdk::wasm_exec_env_t;
use std::fmt;
use libc::gettid;

#[derive(Debug)]
enum CallID {
    ScUnknown,
    ScMmap { grow: i32 },
    ScWritev { fd: i32, iov: i32, iovcnt: i32 },
}

#[derive(Debug)]
struct Access {
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

fn create_call_id(call_id: i32, args: (i32, i32, i32)) -> Option<CallID> {
    match call_id {
        0 => Some(CallID::ScUnknown),
        1 => Some(CallID::ScMmap { grow: args.0 }),
        2 => Some(CallID::ScWritev { fd: args.0, iov: args.1, iovcnt: args.2 }),
        _ => None,
    }
}

pub extern "C" fn wasm_memop_tracedump(_exec_env: wasm_exec_env_t, differ: i32, access_idx: i32, opcode: i32, addr: i32, size: i32, load_value: i64, expected_value: i64) {
    if addr == 0 {
        warn!("[{} | {:#04X}] Access to address [{}::{}] may be invalid", access_idx, opcode, addr, size);
    }
    if differ != 0 {
        let tidval = unsafe { gettid() };
        let access = Access { access_idx, opcode, addr, size, load_value, expected_value };
        debug!("[{}] [Trace MEMOP] {} | Diff? {}", 
            tidval, access, differ);
    }
}

pub extern "C" fn wasm_call_tracedump(_exec_env: wasm_exec_env_t, access_idx: i32, opcode: i32, func_idx: i32, 
        call_id: i32, a1: i64, a2: i64, a3: i64) {
    if opcode != 0x10 {
        warn!("[{} | {:#04X}] Unexpected opcode", access_idx, opcode);
    }
    let tidval = unsafe { gettid() };
    let call_id = create_call_id(call_id, (a1 as i32, a2 as i32, a3 as i32)).unwrap();
    debug!("[{}] [Trace CALL] [{:6} | {:#6X}] for [{:?} | {:3}]",
        tidval, access_idx, opcode, call_id, func_idx); 
}