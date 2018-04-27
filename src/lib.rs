#![no_std]
#![feature(core_intrinsics)]

extern crate byteorder;

pub mod module;
pub mod environment;
pub mod vm;
pub mod error;
pub mod tape;
