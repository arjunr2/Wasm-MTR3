use std::ptr;
use log::{warn, trace};
use libc::{self, c_void};
use std::mem::{size_of, MaybeUninit};
use serde::{Serialize, Deserialize};

use wamr_rust_sdk::{
    wasm_exec_env_t,
    wasm_runtime_addr_app_to_native, wasm_runtime_get_module_inst,
    wasm_runtime_get_exec_env_uid
};

/// Types for Wasm to Native conversion
pub type Addr = *mut c_void;
pub type WasmAddr = u32;

trait WasmPrimitiveType {}
impl WasmPrimitiveType for i32 {}
impl WasmPrimitiveType for i64 {}
impl WasmPrimitiveType for u32 {}
impl WasmPrimitiveType for u64 {}


/* Futex Flags */
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum FutexOp {
    Wait = 0,
    Wake = 1,
    Unknown = -1
}
impl FutexOp {
    pub fn from_i32(op: i32) -> Self {
        // Mask out FUTEX_PRIVATE (bit 7)
        match op & 0x7f {
            0 => FutexOp::Wait,
            1 => FutexOp::Wake,
            _ => FutexOp::Unknown,
        }
    }
}

/// Convert a Wasm address to a native address
pub unsafe fn maddr(exec_env: wasm_exec_env_t, wasm_addr: WasmAddr) -> Addr {
    let native_addr: *mut c_void = unsafe {
        let module_inst = wasm_runtime_get_module_inst(exec_env);
        if wasm_addr == 0 { 
            ptr::null_mut()
        } else {
            wasm_runtime_addr_app_to_native(module_inst, wasm_addr as u64) as *mut c_void
        }
    };
    native_addr
}

/// Iterator over a pointer to a buffer to extract fields from
/// a C-struct encoding in Wasm
struct PtrIter {
    exec_env: wasm_exec_env_t,
    ptr: *mut u8,
    offset: u32,
    size: u32,
}

impl Iterator for PtrIter {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.size {
            None
        } else {
            let s = Some(unsafe { self.ptr.offset(self.offset as isize) });
            self.offset += 1;
            s
        }
    }
}
impl PtrIter {
    pub fn new(exec_env: wasm_exec_env_t, ptr: *mut c_void, size: u32) -> Self {
        PtrIter {
            exec_env: exec_env, 
            ptr: ptr as *mut u8, 
            offset: 0, 
            size: size
        }
    }
    pub unsafe fn advance<T: WasmPrimitiveType>(&mut self) -> T {
        let size = size_of::<T>();
        let offptr = self.ptr.offset(self.offset as isize);
        let mut retval = MaybeUninit::<T>::uninit();
        ptr::copy_nonoverlapping(offptr, retval.as_mut_ptr() as *mut u8, size);
        let _ = self.advance_by(size);
        retval.assume_init()
    }
    pub unsafe fn advance_addr(&mut self) -> Addr {
        let wasmaddr = self.advance::<WasmAddr>();
        maddr(self.exec_env, wasmaddr)
    }
}


/// Generate a native iovec from a WALI iovec
pub unsafe fn get_native_iovec_from_wali(exec_env: wasm_exec_env_t, wasm_iov: WasmAddr, iovcnt: i32) -> Vec<libc::iovec> {
    let mut native_iovs: Vec<libc::iovec> = Vec::with_capacity(iovcnt as usize);
    let wasm_iovptr = maddr(exec_env, wasm_iov);
    if wasm_iovptr.is_null() {
        warn!("Null iovec pointer found");
        return native_iovs;
    }
    let mut it = PtrIter::new(exec_env, wasm_iovptr, 8 * iovcnt as u32);
    for _ in 0..iovcnt {
        let native_iov_elem = libc::iovec {
            iov_base: it.advance_addr() as *mut c_void,
            iov_len: it.advance::<u32>() as usize,
        };
        trace!("Iovec Conversion | Base: {:?}, Len: {}", 
            native_iov_elem.iov_base, native_iov_elem.iov_len);
        native_iovs.push(native_iov_elem);
    }
    native_iovs
}

/// Get the TID of the Wasm executing environment
/// TIDs start with 1 and sequentially increment in order of creation 
#[inline(always)]
pub fn get_wasmtid(exec_env: wasm_exec_env_t) -> u64 {
    // Offset the wasm runtime's internal TID by 1, since it starts with 2
    unsafe { wasm_runtime_get_exec_env_uid(exec_env) - 1 }
}
