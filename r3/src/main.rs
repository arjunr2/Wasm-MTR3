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


pub fn loop_count(contents: Vec<u8>) -> Result<&'static [u8], Box<dyn Error>> {
    let routine = CString::new("loop-count")?;
    let args: [&str;0] = [];
    let args_c: Vec<*const i8> = args.iter().map(
        |s| CString::new(*s).unwrap().as_ptr()).collect();
    //let args_c: Vec<*const u8> = args.iter().map(|s| s.as_ptr()).collect();
    let mut outsize: u32 = 0;
    let outsize_ptr: *mut u32 = &mut outsize;
    let outslice: &[u8];
    unsafe {
        let outbuf: *mut libc::c_char = 
            instrument_module_buffer(contents.as_ptr() as *const libc::c_char, 
                contents.len() as u32, 
                outsize_ptr, 
                routine.as_ptr() as *const libc::c_char, 
                args_c.as_ptr() as *const *const libc::c_char, 
                args_c.len() as u32);
        outslice = slice::from_raw_parts(outbuf as *const u8, outsize as usize);
    };
    println!("Routine: {}", routine.into_string().expect("Routine cannot be converted to String"));
    println!("Insize: {}, Outsize: {}", contents.len(), outsize);
    return Ok(outslice);
}

fn main() -> Result<(), Box<dyn Error>> {
    let contents = fs::read("lua.wasm")?;
    let out_module: &[u8] = loop_count(contents)?;
    fs::write("out.wasm", out_module)?;
    return Ok(());
}
