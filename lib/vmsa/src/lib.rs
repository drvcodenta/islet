#![no_std]
#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(generic_const_exprs)]
#![warn(rust_2018_idioms)]

pub mod address;
pub mod error;
pub mod guard;
pub mod page;
pub mod page_table;

use armv9a::{define_bitfield, define_bits, define_mask};

define_bits!(
    RawGPA, // ref. K6.1.2
    L0Index[47 - 39],
    L1Index[38 - 30],
    L2Index[29 - 21],
    L3Index[20 - 12]
);

impl From<usize> for RawGPA {
    fn from(addr: usize) -> Self {
        Self(addr as u64)
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
    let buffer = alloc::format!($($arg)*);
    let _ = io::stdout().write_all(buffer.as_bytes());
    };
}

#[macro_export]
macro_rules! println {
    () => {crate::print!("\n")};
    ($fmt:expr) => {crate::print!(concat!($fmt, "\n"))};
    ($fmt:expr, $($arg:tt)*) => {crate::print!(concat!($fmt, "\n"), $($arg)*)};
}
