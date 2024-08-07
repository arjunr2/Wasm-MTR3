use log::{debug, info};
use std::io::{self, Write};
use std::fs::File;
use std::collections::{VecDeque, BTreeMap};

use common::trace::*;

use crate::structs::*;

pub fn dump_replay_ops(replay: &BTreeMap<i32, ReplayOp>, outfile: &str) -> Result<(), io::Error> {
    let mut file = File::create(outfile)?;
    for (_access_idx, op) in replay {
        writeln!(file, "{}", op)?;
    }
    info!("Replay output written to {}", outfile);
    Ok(())
}

fn append_vecd_to_map(map: &mut BTreeMap<i32, ReplayOp>, vecd: &mut VecDeque<ReplayOpSingle>) {
    while let Some(call) = vecd.pop_front() {
        // If we see a repeated access_idx, append stores/returns
        if let Some(ref mut replay_op) = map.get_mut(&call.access_idx) {
            replay_op.props.push(call.prop);
        } else {
            // Otherwise, create a new replay op
            map.insert(call.access_idx, ReplayOp {
                access_idx: call.access_idx,
                func_idx: call.func_idx,
                props: vec![call.prop]
            });
        }
    }
}

/// Construct intermediate replay operations from trace to feed into 
/// replay generator
/// RelayOps have their operations stored in trace-observed order
/// i.e if n happened before m in the trace, then op_idx(n) < op_idx(m)
pub fn construct_replay_ops(trace: &Vec<TraceOp>) -> BTreeMap<i32, ReplayOp> {
    let mut replay: BTreeMap<i32, ReplayOp> = BTreeMap::new();

    let mut queued_seq_calls: VecDeque<ReplayOpSingle> = VecDeque::new();

    for trace_op in trace {
        match trace_op {
            TraceOp::CallOp(call) => {
                // Only Generic or Mmap can cause memory stores
                match call.call_id {
                    CallID::ScGeneric | CallID::ScMmap {..} => {
                        debug!("New call --> {:?}; Flushing queue {:?}", call, queued_seq_calls);
                        // Flush queue when we see a new call of this type
                        append_vecd_to_map(&mut replay, &mut queued_seq_calls);
                    }
                    _ => { }
                }
                // All call ops eventually need to be replayed for return value
                queued_seq_calls.push_back(ReplayOpSingle {
                    access_idx: call.access_idx,
                    func_idx: call.func_idx,
                    prop: ReplayOpProp {
                        return_val: call.return_val,
                        call_id: call.call_id,
                        stores: vec![],
                    }
                });
            }
            TraceOp::MemOp(access) => {
                // We currently map all accesses to the last call
                // i.e, the front of queued calls
                if let Some(ref mut target_call) =  queued_seq_calls.front_mut() {
                    target_call.prop.stores.push(ReplayMemStore {
                        addr: access.addr,
                        size: access.size,
                        value: access.load_value
                    });
                } else {
                    panic!("No previous call to map access to in trace");
                }
            }
        }
    };

    // Flush any remaining queued calls
    append_vecd_to_map(&mut replay, &mut queued_seq_calls);

    return replay;
}