//! Utilities for parsing trace files and constructing replay operations from
//! them
use log::{info, trace};
use std::collections::{BTreeMap, BinaryHeap, VecDeque};
use std::fs::File;
use std::io::{self, Write};

use common::trace::*;

use crate::structs::*;

/// Debug method for writing human-readable generated replay operations to
/// `opsfile`
pub fn dump_replay_ops(replay: &BTreeMap<u32, ReplayOp>, opsfile: &str) -> Result<(), io::Error> {
    let log_props_info: BinaryHeap<ReplayPropLogInfo> = replay
        .values()
        .flat_map(|op| {
            op.props.iter().enumerate().map(|(prop_idx, prop)|
                // Require min-heap based sort
                ReplayPropLogInfo {
                    access_idx: op.access_idx,
                    func_idx: op.func_idx,
                    tid: prop.tid,
                    prop_idx: prop_idx as u32,
                    call_id: prop.call_id,
                    return_val: prop.return_val,
                    sync_id: prop.sync_id,
                })
        })
        .collect();

    let mut file = File::create(opsfile)?;
    writeln!(file, "{}", ReplayPropLogInfo::debug_string_header())?;
    for x in log_props_info.into_iter_sorted() {
        writeln!(file, "{}", x)?;
    }
    info!("Replay operation log written to {}", opsfile);
    Ok(())
}

/// Flush a vector of [`ReplayOpSingle`]s to a new or previous [`ReplayOp`]
fn append_vecd_to_map(map: &mut BTreeMap<u32, ReplayOp>, vecd: &mut VecDeque<ReplayOpSingle>) {
    while let Some(opsingle) = vecd.pop_front() {
        // If we see a repeated access_idx, append stores/returns
        if let Some(ref mut replay_op) = map.get_mut(&opsingle.access_idx) {
            replay_op.max_tid = std::cmp::max(replay_op.max_tid, opsingle.prop.tid);
            replay_op.props.push(opsingle.prop);
        } else {
            let max_tid = opsingle.prop.tid;
            // Otherwise, create a new replay op
            map.insert(
                opsingle.access_idx,
                ReplayOp {
                    access_idx: opsingle.access_idx,
                    func_idx: opsingle.func_idx,
                    implicit_sync: opsingle.implicit_sync,
                    props: vec![opsingle.prop],
                    max_tid: max_tid,
                },
            );
        }
    }
}

/// Order replay ops in **ascending order** of tids, followed by **ascending
/// order** of sync_ids for tiebreaking
///
/// ### Design Notes
/// Simplfies static instrumentation
pub fn reorder_replay_ops(replay_ops: &mut BTreeMap<u32, ReplayOp>) {
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
        trace!("Reordered: {:?}", op);
    }
}

/// Construct replay operations ([`ReplayOp`]s) from a Trace to transform/feed
/// into replay generator
///
/// ### Design Notes
/// [`ReplayOp`]s have individual dynamic operations ([`ReplayOpSingle`]s)
/// stored in trace-observed order, i.e., for two operations `n` and `m`:
/// ```text
///  trace[n] → trace[m]  ⇒ replay_idx[n] < replay_idx[m]
/// ```
/// where ⇒ denotes "happened before" relation
pub fn construct_replay_ops(trace: &Vec<TraceOp>) -> BTreeMap<u32, ReplayOp> {
    let mut replay: BTreeMap<u32, ReplayOp> = BTreeMap::new();

    let mut queued_seq_ops: VecDeque<ReplayOpSingle> = VecDeque::new();

    let mut sync_id_global = 0;
    for trace_op in trace {
        match trace_op {
            TraceOp::Call {
                tid,
                access_idx,
                func_idx,
                return_val,
                call_id,
                ..
            } => {
                // Only Generic or Mmap can cause memory stores
                match call_id {
                    CallID::ScGeneric | CallID::ScMmap { .. } => {
                        trace!(
                            "New call --> {} | {:?}; Flushing queue {:?}",
                            *access_idx,
                            *call_id,
                            queued_seq_ops
                        );
                        // Flush queue when we see a new call of this type
                        append_vecd_to_map(&mut replay, &mut queued_seq_ops);
                    }
                    _ => {}
                }
                // All call ops eventually need to be replayed for return value
                queued_seq_ops.push_back(ReplayOpSingle {
                    access_idx: *access_idx,
                    func_idx: *func_idx,
                    implicit_sync: false,
                    prop: ReplayOpProp {
                        tid: *tid,
                        return_val: *return_val,
                        call_id: *call_id,
                        stores: vec![],
                        sync_id: {
                            sync_id_global += 1;
                            sync_id_global
                        },
                    },
                });
            }
            TraceOp::Access {
                tid,
                access_idx,
                opcode,
                addr,
                size,
                load_value,
                differ,
                ..
            }
            | TraceOp::SyncAccess {
                tid,
                access_idx,
                opcode,
                addr,
                size,
                load_value,
                differ,
                ..
            } => {
                // We currently map all differing accesses to the last call
                // i.e, the front of queued calls
                if *differ {
                    if let Some(ref mut target_call) = queued_seq_ops.front_mut() {
                        target_call.prop.stores.push(ReplayMemStore {
                            addr: *addr,
                            size: *size,
                            value: *load_value,
                        });
                    } else {
                        panic!("No previous call to map access to in trace");
                    }
                }
                // Synchronized accesses are treated as ops for ordering
                // We don't flush to map since it's not a call
                if let TraceOp::SyncAccess { .. } = trace_op {
                    trace!(
                        "New sync access --> {:?}; Flushing queue {:?}",
                        *opcode,
                        queued_seq_ops
                    );
                    queued_seq_ops.push_back(ReplayOpSingle {
                        access_idx: *access_idx,
                        func_idx: u32::MAX,
                        implicit_sync: true,
                        prop: ReplayOpProp {
                            tid: *tid,
                            return_val: i64::MAX,
                            call_id: CallID::ScUnknown,
                            stores: vec![],
                            sync_id: {
                                sync_id_global += 1;
                                sync_id_global
                            },
                        },
                    });
                }
            }
        }
    }

    // Flush any remaining queued calls
    append_vecd_to_map(&mut replay, &mut queued_seq_ops);

    return replay;
}
