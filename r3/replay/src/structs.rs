use std::fmt;

use common::trace::{CallID};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ReplayMemStore {
    pub addr: i32,
    pub size: i32,
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
    pub access_idx: i32,
    pub func_idx: i32,
    pub prop: ReplayOpProp,
}

/// A replay operation that aggregates single operation
#[derive(Debug, Clone)]
pub struct ReplayOp {
    pub access_idx: i32,
    pub func_idx: i32,
    pub props: Vec<ReplayOpProp>
}
impl fmt::Display for ReplayOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ReplayOp [{:6} | {:3}] with {:?}", 
            self.access_idx, self.func_idx, self.props)
    }
}


/// C structs used in the FFI to instrumentation library
#[repr(C)]
#[derive(Debug)]
pub struct ReplayOpPropCFFI {
    pub return_val: i64,
    pub call_id: i32,
    pub call_args: [i32; 3],
    pub stores : *const ReplayMemStore,
    pub stores_len: i32,
}

#[repr(C)]
#[derive(Debug)]
pub struct ReplayOpCFFI {
    pub access_idx: i32,
    pub func_idx: i32,
    pub props: *const ReplayOpPropCFFI,
    pub props_len: i32,
}