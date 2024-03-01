use libc::{size_t, c_char};
use std::fs;
use std::error::{Error};

#[link(name = "wasminstrument", kind = "static")]
extern {
    fn instrument_module_buffer(inbuf: *mut libc::c_void, insize: u32, 
        outsize: *mut u32, routine: *const libc::c_char, args: *const *const libc::c_char, num_args: u32) -> *mut libc::c_void;
}


pub fn loop_count(contents: String) {
    let routine: &str = "loop-count";
    let args: [&str;1] = [""];
    let num_args: u32 = 0;
    unsafe {
        instrument_module_buffer(contents.as_ptr(), contents.len(), 
    }
    println!("Routine: {}", routine);
    println!("File content: {}", contents);
}

fn main() -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string("k.txt")?;
    loop_count(contents);
    return Ok(());
}
