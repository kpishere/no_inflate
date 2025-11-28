#![cfg_attr(test, no_std)]
#![deny(warnings)]

extern crate alloc;

// expose the main API
pub mod inflate;

pub use inflate::{inflate_zlib, InflateError};
