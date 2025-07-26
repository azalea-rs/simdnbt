#![doc = include_str!("../README.md")]
#![feature(portable_simd)]
#![feature(array_chunks)]
#![allow(internal_features)]
#![feature(core_intrinsics)]
#![feature(allocator_api)]

#[cfg(not(target_pointer_width = "64"))]
compile_error!("simdnbt only supports 64-bit platforms");

pub mod borrow;
mod common;
mod error;
mod fastvec;
mod mutf8;
pub mod owned;
pub mod raw_list;
mod reader;
pub mod swap_endianness;
mod traits;

pub use error::{DeserializeError, Error};
pub use mutf8::{Mutf8Str, Mutf8String};
pub use simdnbt_derive::*;
pub use traits::{Deserialize, FromNbtTag, Serialize, ToNbtTag};

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use flate2::bufread::GzDecoder;

    fn gzip_decode(src: &[u8]) -> Vec<u8> {
        let mut src_slice = src;
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        decoded_src
    }

    fn test_decodes_equally(src: &[u8]) {
        let nbt_borrow = crate::borrow::read(&mut Cursor::new(src))
            .unwrap()
            .unwrap()
            .as_compound()
            .to_owned();
        let nbt_owned = crate::owned::read(&mut Cursor::new(src))
            .unwrap()
            .unwrap()
            .as_compound();
        // (there's another test in owned to make sure that PartialEq actually works)
        assert_eq!(nbt_borrow, nbt_owned);
    }

    #[test]
    fn complex_player_borrow_and_owned() {
        let src = include_bytes!("../tests/complex_player.dat").to_vec();
        test_decodes_equally(&gzip_decode(&src));
    }
    #[test]
    fn level_borrow_and_owned() {
        let src = include_bytes!("../tests/level.dat").to_vec();
        test_decodes_equally(&gzip_decode(&src));
    }
    #[test]
    fn hypixel_borrow_and_owned() {
        let src = include_bytes!("../tests/hypixel.nbt").to_vec();
        test_decodes_equally(&src);
    }
    #[test]
    fn bigtest_borrow_and_owned() {
        let src = include_bytes!("../tests/bigtest.nbt").to_vec();
        test_decodes_equally(&gzip_decode(&src));
    }
}
