#![doc = include_str!("../README.md")]
#![feature(portable_simd)]
#![feature(array_chunks)]
#![allow(internal_features)]
#![feature(core_intrinsics)]

#[cfg(not(target_pointer_width = "64"))]
compile_error!("simdnbt only supports 64-bit platforms");

pub mod borrow;
mod common;
mod error;
mod mutf8;
pub mod owned;
pub mod raw_list;
mod reader;
pub mod swap_endianness;
mod traits;

pub use error::{DeserializeError, Error};
pub use mutf8::Mutf8Str;
pub use traits::{Deserialize, FromNbtTag, Serialize, ToNbtTag};

pub use simdnbt_derive::*;

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use flate2::bufread::GzDecoder;

    #[test]
    fn complex_player_borrow_and_owned() {
        let src = include_bytes!("../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        let nbt_borrow = crate::borrow::read(&mut Cursor::new(&decoded_src))
            .unwrap()
            .unwrap()
            .as_compound()
            .to_owned();
        let nbt_owned = crate::owned::read(&mut Cursor::new(&decoded_src))
            .unwrap()
            .unwrap()
            .as_compound();

        // (there's another test in owned to make sure that PartialEq actually works)
        assert_eq!(nbt_borrow, nbt_owned);
    }
}
