use libc::{size_t, c_char};
use std::ffi::CString;
use std::fs;
use std::slice;
use std::error::{Error};

#[link(name = "wasminstrument", kind = "static")]
extern {
    fn instrument_module_buffer(inbuf: *const libc::c_char, insize: u32, 
        outsize: *mut u32, routine: *const libc::c_char, args: *const *const libc::c_char, num_args: u32) -> *mut libc::c_char;
}


pub fn instrument_module(contents: Vec<u8>, routine: &str, args: &[&str]) -> Result<&'static [u8], Box<dyn Error>> {
    println!("Routine: {}", routine);

    let c_routine = CString::new(routine)?;
    let args_cstr: Vec<CString> = args.iter().map(
        |s| CString::new(*s).unwrap()).collect();
    let c_args: Vec<*const i8> = args_cstr.iter().map(
        |s| s.as_ptr()).collect();
    let mut outsize: u32 = 0;
    let outsize_ptr: *mut u32 = &mut outsize;
    let outslice: &[u8];
    unsafe {
        let outbuf: *mut libc::c_char = 
            instrument_module_buffer(contents.as_ptr() as *const libc::c_char, 
                contents.len() as u32, 
                outsize_ptr, 
                c_routine.as_ptr() as *const libc::c_char, 
                c_args.as_ptr() as *const *const libc::c_char, 
                c_args.len() as u32);
        outslice = slice::from_raw_parts(outbuf as *const u8, outsize as usize);
    };
    println!("Insize: {}, Outsize: {}", contents.len(), outsize);
    return Ok(outslice);
}

fn main() -> Result<(), Box<dyn Error>> {
    let contents = fs::read("lua.wasm")?;
    let routine = "memaccess-stochastic";
    let args: [&str;2] = ["30", "1"];

    let out_module: &[u8] = instrument_module(contents, routine, &args)?;
    fs::write("out.wasm", out_module)?;
    return Ok(());
}
