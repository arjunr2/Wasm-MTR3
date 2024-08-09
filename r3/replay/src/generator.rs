use log::{debug, info};
use std::error::{Error};
use libc::{c_void};
use std::mem::ManuallyDrop;

use crate::structs::*;

use common::trace::{get_instrumentation_call_id};
use common::instrument::{InstrumentArgs, instrument_module, destroy_instrument_module};

use std::collections::BTreeMap;

struct FFIManualDropData {
    ffi_props_all: Vec<Vec<ReplayOpPropCFFI>>,
}

/// To generate this C-like FFI struct, we need to have manually
/// dropped data that the user is required to drop after use
fn generate_ffi_ops(replay_ops: &BTreeMap<u32, ReplayOp>) -> 
        (Vec<ReplayOpCFFI>, ManuallyDrop<FFIManualDropData>) {
    let mut ffi_ops: Vec<ReplayOpCFFI> = Vec::new();
    let mut ffi_manual_drop = ManuallyDrop::new(FFIManualDropData {
        ffi_props_all: Vec::new(),
    });
    for (_access_idx, op) in replay_ops {
        //let mut ffi_props: Vec<ReplayOpPropCFFI> = Vec::new();
        ffi_manual_drop.ffi_props_all.push(Vec::new());
        {
            let ffi_props: &mut Vec<ReplayOpPropCFFI> = ffi_manual_drop.ffi_props_all.last_mut().unwrap();
            for prop in &op.props {
                let (ffi_call_id, ffi_call_args) = get_instrumentation_call_id(&prop.call_id);
                ffi_props.push(ReplayOpPropCFFI {
                    return_val: prop.return_val,
                    call_id: ffi_call_id,
                    call_args: ffi_call_args,
                    stores: prop.stores.as_ptr(),
                    num_stores: prop.stores.len() as u32,
                });
            }
            // Push the actual Op data
            ffi_ops.push(ReplayOpCFFI {
                access_idx: op.access_idx,
                func_idx: op.func_idx,
                props: ffi_props.as_ptr(),
                num_props: ffi_props.len() as u32,
            });
        }
    }
    (ffi_ops, ffi_manual_drop)
}

pub fn generate_replay_file(replay_ops: &BTreeMap<u32, ReplayOp>, wasmbin: &Vec<u8>, outfile: &str) -> Result<(), Box<dyn Error>> {
    let (ffi_ops, mut ffi_manual_drop) = generate_ffi_ops(replay_ops);
    for op in &ffi_ops {
        debug!("{}", op);
    }
    info!("Generating replay file {}", outfile);
    let inst_module: &[u8] = instrument_module(wasmbin, "r3-replay-generator", InstrumentArgs::AnonArr(ffi_ops.as_ptr() as *const c_void, ffi_ops.len() as u32))?;
    // Drop the manually managed C FFI replay-op data
    unsafe {
        ManuallyDrop::drop(&mut ffi_manual_drop);
    }
    destroy_instrument_module(inst_module);
    Ok(())
}