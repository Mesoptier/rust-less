//#![no_std]
extern crate alloc;

#[cfg(test)]
extern crate test_case;

mod ast;
mod parser;

pub use parser::{parse_stylesheet};