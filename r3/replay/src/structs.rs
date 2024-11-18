//! Datatypes used to represent replay operations and their properties
use std::fmt;

use common::trace::CallID;

/// Represents a memory store operation to replay
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ReplayMemStore {
    pub addr: i32,
    pub size: u32,
    pub value: i64,
}

/// Dynamic properties of a **single** dynamic replay operation
#[derive(Debug, Clone)]
pub struct ReplayOpProp {
    pub tid: u64,
    pub return_val: i64,
    pub call_id: CallID,
    pub stores: Vec<ReplayMemStore>,
    /// Used for synchronization calls to enforce ordering
    pub sync_id: u64,
}
impl fmt::Display for ReplayOpProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Props [ {:#X} <-- {:?} --> {:?}]",
            self.return_val, self.call_id, self.stores
        )
    }
}

/// Dynamic + static properties of a **single** dynamic replay operation
///
/// Single operations can either point to function calls or
/// implicit synchronization points (`implicit_sync=true`) from Wasm (e.g.
/// cmpxchg)
#[derive(Debug, Clone)]
pub struct ReplayOpSingle {
    pub access_idx: u32,
    pub func_idx: u32,
    pub implicit_sync: bool,
    pub prop: ReplayOpProp,
}

/// A replay operation that aggregates [`ReplayOpSingle`] to a static code
/// location into a single operation.
///
/// ### Design Notes
/// `access_idx` specifies the static code location. This is the most format to
/// enable static instrumentation for replay generation
#[derive(Debug, Clone)]
pub struct ReplayOp {
    pub access_idx: u32,
    pub func_idx: u32,
    pub implicit_sync: bool,
    pub props: Vec<ReplayOpProp>,
    pub max_tid: u64,
}
impl ReplayOp {
    /// Returns the total number of memory stores across all properties
    pub fn total_stores(&self) -> usize {
        let mut total_stores = 0;
        for prop in &self.props {
            total_stores += prop.stores.len();
        }
        total_stores
    }
}
impl fmt::Display for ReplayOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fx = self.func_idx.to_string();
        write!(
            f,
            "ReplayOp [{:6} | {:3}] with PropOp[{}](stores: {})",
            self.access_idx,
            if self.implicit_sync { "SY" } else { &fx },
            self.props.len(),
            self.total_stores()
        )
    }
}

/// [`ReplayOpProp`]'s representation for exchange over FFI to C++
/// instrumentation library
#[repr(C)]
#[derive(Debug)]
pub struct ReplayOpPropCFFI {
    pub tid: u64,
    pub return_val: i64,
    pub call_id: u32,
    pub call_args: [i64; 3],
    pub stores: *const ReplayMemStore,
    pub num_stores: u32,
    pub sync_id: u64,
}

/// [`ReplayOp`]'s representation for exchange over FFI to C++ instrumentation
/// library
///
/// Props are assumed to be ordered by TID and then by sync_id
#[repr(C)]
#[derive(Debug)]
pub struct ReplayOpCFFI {
    pub access_idx: u32,
    pub func_idx: u32,
    pub implicit_sync: u32,
    pub props: *const ReplayOpPropCFFI,
    pub num_props: u32,
    pub max_tid: u64,
}
impl ReplayOpCFFI {
    /// Returns the total number of memory stores across all properties
    pub fn total_stores(&self) -> usize {
        let mut total_stores = 0;
        for i in 0..self.num_props as usize {
            let prop = unsafe { &*self.props.offset(i as isize) };
            total_stores += prop.num_stores as usize;
        }
        total_stores
    }
}
impl fmt::Display for ReplayOpCFFI {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ReplayOp [{:6} | {:3}] with PropOp[{}](stores: {}) {}",
            self.access_idx,
            self.func_idx,
            self.num_props,
            self.total_stores(),
            if self.implicit_sync != 0 {
                "[SYNC]"
            } else {
                ""
            }
        )
    }
}
