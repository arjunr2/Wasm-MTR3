use libc::{c_char};

use std::ffi::CString;
use std::slice;
use std::error::{Error};
use log::{info};

#[link(name = "wasminstrument", kind = "static")]
extern {
    fn instrument_module_buffer(inbuf: *const c_char, insize: u32, 
        outsize: *mut u32, routine: *const c_char, args: *const *const c_char, num_args: u32) -> *mut c_char;

    fn destroy_file_buf(buf: *const c_char) -> ();
}


pub fn instrument_module(contents: Vec<u8>, routine: &str, args: &[&str]) -> Result<&'static [u8], Box<dyn Error>> {
    let c_routine = CString::new(routine)?;
    let args_cstr: Vec<CString> = args.iter().map(
        |s| CString::new(*s).unwrap()).collect();
    let c_args: Vec<*const i8> = args_cstr.iter().map(
        |s| s.as_ptr()).collect();
    let mut outsize: u32 = 0;
    let outsize_ptr: *mut u32 = &mut outsize;
    let outslice: &[u8];
    unsafe {
        let outbuf: *mut c_char = 
            instrument_module_buffer(contents.as_ptr() as *const c_char, 
                contents.len() as u32, 
                outsize_ptr, 
                c_routine.as_ptr() as *const c_char, 
                c_args.as_ptr() as *const *const c_char, 
                c_args.len() as u32);
        outslice = slice::from_raw_parts(outbuf as *const u8, outsize as usize);
    };
    info!("Instrument | Insize: {}, Outsize: {}", contents.len(), outsize);
    return Ok(outslice);
}

pub fn destroy_instrument_module(contents: &[u8]) -> () {
    unsafe {
        destroy_file_buf(contents.as_ptr() as *const c_char);
    }
}


