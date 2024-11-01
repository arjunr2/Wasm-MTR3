use log::{debug, info, trace};
use std::io::{self, Write};
use std::fs::File;
use std::collections::{VecDeque, BTreeMap};

use common::trace::*;

use crate::structs::*;

pub fn dump_replay_ops(replay: &BTreeMap<u32, ReplayOp>, outfile: &str) -> Result<(), io::Error> {
    let mut file = File::create(outfile)?;
    for (_access_idx, op) in replay {
        writeln!(file, "{}", op)?;
    }
    info!("Replay operation log written to {}", outfile);
    Ok(())
}

fn append_vecd_to_map(map: &mut BTreeMap<u32, ReplayOp>, vecd: &mut VecDeque<ReplayOpSingle>) {
    while let Some(call) = vecd.pop_front() {
        // If we see a repeated access_idx, append stores/returns
        if let Some(ref mut replay_op) = map.get_mut(&call.access_idx) {
            replay_op.max_tid = std::cmp::max(replay_op.max_tid, call.prop.tid);
            replay_op.props.push(call.prop);
        } else {
            let max_tid = call.prop.tid;
            // Otherwise, create a new replay op
            map.insert(call.access_idx, ReplayOp {
                access_idx: call.access_idx,
                func_idx: call.func_idx,
                props: vec![call.prop],
                max_tid: max_tid
            });
        }
    }
}

/// Reorder replay ops with tids first and then sync_ids; simplfies instrumentation
fn reorder_replay_ops(replay_ops: &mut BTreeMap<u32, ReplayOp>) {
    for (_, op) in replay_ops.iter_mut() {
        op.props.sort_by(|a, b| {
            if a.tid == b.tid {
                a.sync_id.cmp(&b.sync_id)
            } else {
                a.tid.cmp(&b.tid)
            }
        });
    }
    for (_, op) in replay_ops.iter_mut() {
        debug!("Reordered: {:?}", op);
    }
}

/// Construct intermediate replay operations from trace to feed into 
/// replay generator
/// ReplayOps have their operations stored in trace-observed order
/// i.e if n happened before m in the trace, then op_idx(n) < op_idx(m)
pub fn construct_replay_ops(trace: &Vec<TraceOp>) -> BTreeMap<u32, ReplayOp> {
    let mut replay: BTreeMap<u32, ReplayOp> = BTreeMap::new();

    let mut queued_seq_calls: VecDeque<ReplayOpSingle> = VecDeque::new();

    let mut sync_id_global = 0;
    for trace_op in trace {
        match trace_op {
            TraceOp::Call{tid, access_idx, func_idx, return_val, call_id, ..} => {
                // Only Generic or Mmap can cause memory stores
                match call_id {
                    CallID::ScGeneric | CallID::ScMmap {..} => {
                        trace!("New call --> {} | {:?}; Flushing queue {:?}", 
                            *access_idx, *call_id, queued_seq_calls);
                        // Flush queue when we see a new call of this type
                        append_vecd_to_map(&mut replay, &mut queued_seq_calls);
                    }
                    _ => { }
                }
                // All call ops eventually need to be replayed for return value
                queued_seq_calls.push_back(ReplayOpSingle {
                    access_idx: *access_idx,
                    func_idx: *func_idx,
                    prop: ReplayOpProp {
                        tid: *tid,
                        return_val: *return_val,
                        call_id: *call_id,
                        stores: vec![],
                        sync_id: {
                            sync_id_global += 1;
                            sync_id_global
                        }
                    }
                });
            }
            TraceOp::Access{addr, size, load_value, ..} => {
                // We currently map all accesses to the last call
                // i.e, the front of queued calls
                if let Some(ref mut target_call) =  queued_seq_calls.front_mut() {
                    target_call.prop.stores.push(ReplayMemStore {
                        addr: *addr,
                        size: *size,
                        value: *load_value
                    });
                } else {
                    panic!("No previous call to map access to in trace");
                }
            }
            TraceOp::ContextSwitch{..} => {
                debug!("Got context switch op");
            }
        }
    };

    // Flush any remaining queued calls
    append_vecd_to_map(&mut replay, &mut queued_seq_calls);

    // Reorder replay ops to order by tids first and then sync_ids
    reorder_replay_ops(&mut replay); 

    return replay;
}