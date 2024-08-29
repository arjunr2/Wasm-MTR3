use log::{debug, warn, info, log_enabled};
use log::Level::Trace;
use std::io::{self, Write, Seek};
use std::fs::{File, remove_file};
use std::sync::{LazyLock, Mutex};
use std::path::PathBuf;
use wamr_rust_sdk::wasm_exec_env_t;
use libc::gettid;
use tempfile::{env};
use postcard;
use uuid::Uuid;
use once_cell::sync::Lazy;

use common::trace::*;
use common::wasm2native::*;

static TMP_FILEPATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut temppath = env::temp_dir();
    temppath.push(Uuid::new_v4().to_string());
    info!("Intermediate tracefile: {:?}", temppath);
    temppath
});

/* Used by Wasm engine to log TraceOps */
pub static TRACEOP_FILE: LazyLock<Mutex<File>> = LazyLock::new(|| {
    Mutex::new(File::create(&*TMP_FILEPATH).unwrap())
});

/* Initialize the temporary file name (UUID) */
pub fn initialize_tmpfile_name() {
    let _ = &*TMP_FILEPATH;
}

/* Record Op to global trace */
fn append_traceop(op: TraceOp) {
    let file = &mut *(TRACEOP_FILE.lock().unwrap());
    postcard::to_io(&op, file).unwrap();
}

fn is_at_eof(mut file: &File) -> io::Result<bool> {
    let current_pos = file.stream_position()?;
    let file_len = file.metadata()?.len();
    Ok(current_pos == file_len)
}    

/* Read the traceops from the tmpfile, and generate finalized trace data */
pub fn dump_global_trace(tracefile: &String, sha256: &str) -> io::Result<()>{
    let mut dumpfile = File::create(tracefile)?;
    let traceop_file = File::open(&*TMP_FILEPATH)?;
    let mut trace_data = TraceData {
        sha256: sha256,
        trace: vec![],
    };

    /* Read each traceop from the intermediate file and convert to final trace format*/
    while !is_at_eof(&traceop_file)? {
        let top = postcard::from_io((&traceop_file, &mut [0; 0])).unwrap();
        trace_data.trace.push(top.0);
    }
    let ser = trace_data.serialize();
    dumpfile.write_all(&ser)?;

    /* Cleanup the temporary file */
    remove_file(&*TMP_FILEPATH)?;

    /* Verify serialization can be effectively deserialized */
    let deserialized = TraceData::deserialize(&ser, None);
    assert_eq!(*trace_data.trace, deserialized.trace);
    Ok(())
}



/* Wasm Engine Hook: Records MemOps */
pub extern "C" fn wasm_memop_tracedump(_exec_env: wasm_exec_env_t, differ: i32, access_idx: u32, opcode: i32, addr: i32, size: u32, load_value: i64, expected_value: i64) {
    if addr == 0 {
        warn!("[{} | {:#04X}] Access to address [{}::{}] may be invalid", access_idx, opcode, addr, size);
    }
    if differ != 0 {
        let tidval = unsafe { gettid() };
        let access = Access { access_idx, opcode, addr, size, load_value, expected_value };
        debug!("[{}] [Trace MEMOP] {} | Diff? {}", tidval, access, differ);
        /* Add to trace here */
        append_traceop(TraceOp::MemOp(access));
    }
}

/* Wasm Engine Hook: Records CallOps */
pub extern "C" fn wasm_call_tracedump(exec_env: wasm_exec_env_t, access_idx: u32, opcode: i32, func_idx: u32, 
        call_id: u32, return_val: i64, a1: i64, a2: i64, a3: i64) {
    if opcode != 0x10 {
        warn!("[{} | {:#04X}] Unexpected opcode", access_idx, opcode);
    }
    let tidval = unsafe { gettid() };
    let call_id = create_call_id(call_id, [a1, a2, a3]).unwrap();
    let call_trace = Call { access_idx, opcode, func_idx, return_val, call_id };
    debug!("[{}] [Trace CALL] {}", tidval, call_trace); 
    if log_enabled!(Trace) {
        if let CallID::ScWritev {fd: _, iov, iovcnt} = call_id {
            let _ = unsafe {
                get_native_iovec_from_wali(exec_env, iov as WasmAddr, iovcnt as i32);
            };
        }
    }
    /* Add to trace here */
    append_traceop(TraceOp::CallOp(call_trace));
}