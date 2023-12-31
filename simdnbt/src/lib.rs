//! an unnecessarily fast nbt decoder.
//!
//! afaik, this is currently the fastest nbt decoder in existence.
//!
//! ```
//! use simdnbt::borrow::Nbt;
//! use std::io::Cursor;
//!
//! let nbt = Nbt::read(&mut Cursor::new(include_bytes!("../tests/hello_world.nbt"))).unwrap().unwrap();
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
pub mod owned;
pub mod raw_list;
pub mod swap_endianness;
mod traits;

pub use error::{DeserializeError, Error};
pub use mutf8::Mutf8Str;
pub use traits::{Deserialize, FromNbtTag, Serialize, ToNbtTag};

pub use simdnbt_derive::*;
