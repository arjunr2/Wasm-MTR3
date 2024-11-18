//! Utilities to implement foreign function interface for trace recording
use log::Level::Trace;
use log::{debug, info, log_enabled, warn};
use once_cell::sync::Lazy;
use postcard;
use std::fs::{remove_file, File};
use std::io::{self, BufWriter, Seek, Write};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use tempfile::env;
use uuid::Uuid;
use wamr_rust_sdk::wasm_exec_env_t;

use common::trace::*;
use common::wasm2native::*;

/// A Lazy-initialized temporary disk-backed filepath
static TMP_FILEPATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut temppath = env::temp_dir();
    temppath.push(Uuid::new_v4().to_string());
    info!("Intermediate tracefile: {:?}", temppath);
    temppath
});

/// A Lazy-initialized writer to a temporary disk-backed file for storing
/// intermediate traceops
pub static TRACEOP_FILE: LazyLock<Mutex<BufWriter<File>>> =
    LazyLock::new(|| Mutex::new(BufWriter::new(File::create(&*TMP_FILEPATH).unwrap())));

/// Initialize the temporary file name
pub fn initialize_tmpfile_name() {
    let _ = &*TMP_FILEPATH;
}

/// Add a [TraceOp] to recorded trace
fn append_traceop(op: TraceOp) {
    let file = &mut *(TRACEOP_FILE.lock().unwrap());
    postcard::to_io(&op, file).unwrap();
}

/// Check if file is at EOF
fn is_at_eof(mut file: &File) -> io::Result<bool> {
    let current_pos = file.stream_position()?;
    let file_len = file.metadata()?.len();
    Ok(current_pos == file_len)
}

/// Generates the finalized trace to `tracefile` with the `sha256` digest by
/// aggregating intermediate generated traceops
pub fn dump_global_trace(tracefile: &String, sha256: &str) -> io::Result<()> {
    let mut dumpfile = File::create(tracefile)?;
    let traceop_file = File::open(&*TMP_FILEPATH)?;
    let mut trace_data = TraceData {
        sha256: sha256,
        trace: vec![],
    };

    // Read each traceop from the intermediate file and convert to final trace
    // format
    while !is_at_eof(&traceop_file)? {
        let top = postcard::from_io((&traceop_file, &mut [0; 0])).unwrap();
        trace_data.trace.push(top.0);
    }
    let ser = trace_data.serialize();
    dumpfile.write_all(&ser)?;

    // Cleanup the temporary file
    remove_file(&*TMP_FILEPATH)?;

    // Verify serialization can be effectively deserialized
    let deserialized = TraceData::deserialize(&ser, None);
    assert_eq!(*trace_data.trace, deserialized.trace);
    Ok(())
}

/// Wasm Record-FFI -- Recording memory operations to Trace
pub extern "C" fn wasm_memop_tracedump(
    exec_env: wasm_exec_env_t,
    differ: i32,
    access_idx: u32,
    opcode: i32,
    addr: i32,
    size: u32,
    load_value: i64,
    expected_value: i64,
    is_sync_op: i32,
) {
    let tid = get_wasmtid(exec_env);
    if addr == 0 {
        warn!(
            "[{} | {:#04X}] Access to address [{}::{}] may be invalid",
            access_idx, opcode, addr, size
        );
    }
    // Synchronization operations are always traced
    if is_sync_op != 0 {
        let sync_access = TraceOp::SyncAccess {
            tid,
            access_idx,
            opcode,
            addr,
            size,
            load_value,
            expected_value,
            differ: differ != 0,
        };
        debug!("[{:>18}] [Trace SYNCACCESS] {}", tid, sync_access);
        append_traceop(sync_access);
    }
    // Non-Synchronized operations are only traced when diff
    else if differ != 0 {
        let access = TraceOp::Access {
            tid,
            access_idx,
            opcode,
            addr,
            size,
            load_value,
            expected_value,
            differ: differ != 0,
        };
        debug!("[{:>18}] [Trace ACCESS] {} | Diff? {}", tid, access, differ);
        append_traceop(access);
    }
}

/// Wasm Record-FFI -- Recording function call operations to Trace
///
/// Currently only looks at import calls
pub extern "C" fn wasm_call_tracedump(
    exec_env: wasm_exec_env_t,
    access_idx: u32,
    opcode: i32,
    func_idx: u32,
    call_id: u32,
    return_val: i64,
    a1: i64,
    a2: i64,
    a3: i64,
) {
    let tid = get_wasmtid(exec_env);
    if opcode != 0x10 {
        warn!("[{} | {:#04X}] Unexpected opcode", access_idx, opcode);
    }
    let call_id = CallID::from_parts(call_id, [a1, a2, a3]).unwrap();
    let call_trace = TraceOp::Call {
        tid,
        access_idx,
        opcode,
        func_idx,
        return_val,
        call_id,
    };
    debug!("[{:>18}] [Trace CALL] {}", tid, call_trace);
    if log_enabled!(Trace) {
        if let CallID::ScWritev { iov, iovcnt, .. } = call_id {
            let _ = unsafe {
                get_native_iovec_from_wali(exec_env, iov as WasmAddr, iovcnt as i32);
            };
        }
    }
    append_traceop(call_trace);
}
