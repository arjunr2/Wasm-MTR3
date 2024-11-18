//! FFI utilities for accessing the [`wasm-instrument`](https://github.com/arjunr2/wasm-instrument)
//! C++ instrumentation API
use libc::{c_char, c_void};

use log::info;
use std::error::Error;
use std::ffi::CString;
use std::slice;

#[link(name = "wasminstrument", kind = "static")]
extern "C" {
    /// API to instrument a module
    fn instrument_module_buffer(
        inbuf: *const c_char,
        insize: u32,
        outsize: *mut u32,
        routine: *const c_char,
        args: *const c_void,
        num_args: u32,
        flags: i64,
    ) -> *mut c_char;

    /// API to cleanup allocations from [instrument_module_buffer]
    fn destroy_file_buf(buf: *const c_char) -> ();
}

/// Arguments for the instrumentation routine
pub enum InstrumentArgs<'a> {
    Generic(&'a [&'a str]),
    AnonArr(*const c_void, u32, i64),
}

/// Convenient Rust wrapper method to instrument a module
///
/// Instrumentation is performed in-place.
///
/// **NOTE**: Remember to call [destroy_instrument_module] to prevent memory
/// leaks after you are done with the output buffer
///
/// ### Usage
/// ```rust
/// let contents = fs::read(file)?;
/// let routine = "r3-record";
/// let args = InstrumentArgs::Generic(&[]);
/// let outbuf = instrument_module(&contents, routine, args)?;
/// // ...
/// // ...
/// destroy_instrument_module(outbuf);
/// ```
pub fn instrument_module(
    contents: &Vec<u8>,
    routine: &str,
    args: InstrumentArgs,
) -> Result<&'static [u8], Box<dyn Error>> {
    let c_routine = CString::new(routine)?;
    let (c_args_ptr, c_args_len, c_args_flags) = match args {
        InstrumentArgs::Generic(ax) => {
            let args_cstr: Vec<CString> = ax.iter().map(|s| CString::new(*s).unwrap()).collect();
            let c_args: Vec<*const i8> = args_cstr.iter().map(|s| s.as_ptr()).collect();
            (c_args.as_ptr() as *const c_void, c_args.len() as u32, 0)
        }
        InstrumentArgs::AnonArr(ax, num_ax, flags) => (ax, num_ax, flags),
    };
    let mut outsize: u32 = 0;
    let outsize_ptr: *mut u32 = &mut outsize;
    let outslice: &[u8];
    unsafe {
        let outbuf: *mut c_char = instrument_module_buffer(
            contents.as_ptr() as *const c_char,
            contents.len() as u32,
            outsize_ptr,
            c_routine.as_ptr() as *const c_char,
            c_args_ptr,
            c_args_len,
            c_args_flags,
        );
        outslice = slice::from_raw_parts(outbuf as *const u8, outsize as usize);
    };
    info!(
        "Instrument | Insize: {}, Outsize: {}",
        contents.len(),
        outsize
    );
    return Ok(outslice);
}

/// Convenient Rust wrapper to destroy the output buffer from
/// [instrument_module]
pub fn destroy_instrument_module(contents: &[u8]) -> () {
    unsafe {
        destroy_file_buf(contents.as_ptr() as *const c_char);
    }
}
