use log::info;
use std::io::{self, Error};

use crate::structs::*;

use common::trace::{CallID, get_instrumentation_call_id};

use std::collections::BTreeMap;

fn generate_ffi_ops(replay_ops: &BTreeMap<i32, ReplayOp>) -> Vec<ReplayOpCFFI> {
    let mut ffi_ops: Vec<ReplayOpCFFI> = Vec::new();
    for (_access_idx, op) in replay_ops {
        let mut ffi_props: Vec<ReplayOpPropCFFI> = Vec::new();
        for prop in &op.props {
            let (ffi_call_id, ffi_call_args) = get_instrumentation_call_id(&prop.call_id);
            ffi_props.push(ReplayOpPropCFFI {
                return_val: prop.return_val,
                call_id: ffi_call_id,
                call_args: ffi_call_args,
                stores: prop.stores.as_ptr(),
                stores_len: prop.stores.len() as i32,
            });
        }

        ffi_ops.push(ReplayOpCFFI {
            access_idx: op.access_idx,
            func_idx: op.func_idx,
            props: ffi_props.as_ptr(),
            props_len: ffi_props.len() as i32,
        });
    }
    ffi_ops
}

pub fn generate_replay_file(replay_ops: &BTreeMap<i32, ReplayOp>, wasmbin: &Vec<u8>, outfile: &str) -> Result<(), io::Error> {
    info!("Generating replay file {}", outfile);
    let ffi_ops = generate_ffi_ops(replay_ops);
    println!("FFI ops: {:?}", ffi_ops);
    Ok(())
}