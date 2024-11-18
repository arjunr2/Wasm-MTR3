//! Utilities for generating replay instrumentation (over FFI to C++ library)
use libc::c_void;
use log::{debug, info};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::mem::ManuallyDrop;

use crate::structs::*;

use common::instrument::{destroy_instrument_module, instrument_module, InstrumentArgs};

use std::collections::BTreeMap;

/// Container for hosting all dynamic replay properties
///
/// ### Design Note
/// Rust will drop all data once they are out of local scope so raw FFI pointers
/// will not stay after allocation if the instrumentation is called in disjoint
/// scope.
/// This struct allows us to manually control when the data is dropped (which is
/// done after instrumentation is complete)
struct FFIManualDropData {
    ffi_props_all: Vec<Vec<ReplayOpPropCFFI>>,
}

/// To generate this C-like FFI struct, we need to have manually
/// dropped data that the user is required to drop after use
fn generate_ffi_ops(
    replay_ops: &BTreeMap<u32, ReplayOp>,
) -> (Vec<ReplayOpCFFI>, ManuallyDrop<FFIManualDropData>) {
    let mut ffi_ops: Vec<ReplayOpCFFI> = Vec::new();
    let mut ffi_manual_drop = ManuallyDrop::new(FFIManualDropData {
        ffi_props_all: Vec::new(),
    });
    for (_access_idx, op) in replay_ops {
        // let mut ffi_props: Vec<ReplayOpPropCFFI> = Vec::new();
        ffi_manual_drop.ffi_props_all.push(Vec::new());
        {
            let ffi_props: &mut Vec<ReplayOpPropCFFI> =
                ffi_manual_drop.ffi_props_all.last_mut().unwrap();
            for prop in &op.props {
                let (ffi_call_id, ffi_call_args) = prop.call_id.to_parts();
                ffi_props.push(ReplayOpPropCFFI {
                    tid: prop.tid,
                    return_val: prop.return_val,
                    call_id: ffi_call_id,
                    call_args: ffi_call_args,
                    stores: prop.stores.as_ptr(),
                    num_stores: prop.stores.len() as u32,
                    sync_id: prop.sync_id,
                });
            }
            // Push the actual Op data
            ffi_ops.push(ReplayOpCFFI {
                access_idx: op.access_idx,
                func_idx: op.func_idx,
                implicit_sync: op.implicit_sync as u32,
                props: ffi_props.as_ptr(),
                num_props: ffi_props.len() as u32,
                max_tid: op.max_tid,
            });
        }
    }
    (ffi_ops, ffi_manual_drop)
}

/// Generate a replay file by instrumenting the original wasm binary with replay
/// operations
pub fn generate_replay_file(
    replay_ops: &BTreeMap<u32, ReplayOp>,
    wasmbin: &Vec<u8>,
    outfile: &str,
    debug: bool,
) -> Result<(), Box<dyn Error>> {
    let (ffi_ops, mut ffi_manual_drop) = generate_ffi_ops(replay_ops);
    for op in &ffi_ops {
        debug!("{}", op);
    }
    info!("Generating replay file from input wasm binary");
    let replay_module: &[u8] = instrument_module(
        wasmbin,
        "r3-replay-generator",
        InstrumentArgs::AnonArr(
            ffi_ops.as_ptr() as *const c_void,
            ffi_ops.len() as u32,
            debug as i64,
        ),
    )?;
    // Drop the manually managed C FFI replay-op data
    unsafe {
        ManuallyDrop::drop(&mut ffi_manual_drop);
    }

    // Write the instrumented module to file
    let mut file = File::create(outfile)?;
    file.write_all(replay_module)?;
    info!("Wrote replay file to {}", outfile);

    destroy_instrument_module(replay_module);
    Ok(())
}
