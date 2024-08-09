use std::fmt;

use common::trace::{CallID};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ReplayMemStore {
    pub addr: i32,
    pub size: u32,
    pub value: i64,
}

#[derive(Debug, Clone)]
pub struct ReplayOpProp {
    pub return_val: i64,
    pub call_id: CallID,
    pub stores : Vec<ReplayMemStore>
}
impl fmt::Display for ReplayOpProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Props [ {:#X} <-- {:?} --> {:?}]", 
            self.return_val, self.call_id, self.stores)
    }
}

#[derive(Debug, Clone)]
pub struct ReplayOpSingle {
    pub access_idx: u32,
    pub func_idx: u32,
    pub prop: ReplayOpProp,
}

/// A replay operation that aggregates single operation
#[derive(Debug, Clone)]
pub struct ReplayOp {
    pub access_idx: u32,
    pub func_idx: u32,
    pub props: Vec<ReplayOpProp>
}
impl ReplayOp {
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
        write!(f, "ReplayOp [{:6} | {:3}] with PropOp[{}](stores: {})", 
            self.access_idx, self.func_idx, self.props.len(), self.total_stores())
    }
}


/// C structs used in the FFI to instrumentation library
#[repr(C)]
#[derive(Debug)]
pub struct ReplayOpPropCFFI {
    pub return_val: i64,
    pub call_id: u32,
    pub call_args: [i64; 3],
    pub stores : *const ReplayMemStore,
    pub num_stores: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct ReplayOpCFFI {
    pub access_idx: u32,
    pub func_idx: u32,
    pub props: *const ReplayOpPropCFFI,
    pub num_props: u32,
}
impl ReplayOpCFFI {
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
        write!(f, "ReplayOp [{:6} | {:3}] with PropOp[{}](stores: {})", 
            self.access_idx, self.func_idx, self.num_props, self.total_stores())
    }
}