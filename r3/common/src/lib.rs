//! Common library utilities used by [`record`](../record/index.html),
//! [`replay`](../replay/index.html), and [`runner`](../runner/index.html).
#![feature(iter_advance_by)]

pub mod instrument;
pub mod trace;
pub mod wasm2native;
