use log::{info};

pub extern "C" fn wasm_tracedump(differ: i32, access_idx: i32, opcode: i32, addr: i32, size: i32, load_value: i32) {
    //info!("Tracepoint hit! Differ: {}, Access: {}, Opcode: {}, 
    //    Address: {}, Size: {}, Value: {}", differ, access_idx, opcode, addr, size, load_value);
}


