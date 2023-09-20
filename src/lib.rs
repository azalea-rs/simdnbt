//! an unnecessarily fast nbt decoder.
//!
//! afaik, this is currently the fastest nbt decoder in existence.
//!
//! ```
//! use simdnbt::borrow::Nbt;
//! use std::io::Cursor;
//!
//! let nbt = Nbt::new(&mut Cursor::new(include_bytes!("../tests/hello_world.nbt"))).unwrap().unwrap();
//! assert_eq!(nbt.name().to_str(), "hello world");
//! assert_eq!(nbt.string("name").unwrap().to_str(), "Bananrama");
//! ```

#![feature(portable_simd)]
#![feature(array_chunks)]
#![feature(split_array)]

pub mod borrow;
mod common;
mod error;
mod mutf8;

pub use error::Error;
pub use mutf8::Mutf8Str;
