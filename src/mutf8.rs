//! The string representation used in NBT.

use std::{
    borrow::{Borrow, Cow},
    fmt, mem,
    ops::Deref,
    simd::prelude::*,
};

/// A M-UTF8 string slice. This is how strings are represented internally in NBT.
#[derive(Debug, Eq, PartialEq)]
pub struct Mutf8Str {
    pub(crate) slice: [u8],
}
/// An owned M-UTF8 string.
#[derive(Debug, Eq, PartialEq)]
pub struct Mutf8String {
    vec: Vec<u8>,
}

#[inline]
fn is_plain_ascii(slice: &[u8]) -> bool {
    let mut is_plain_ascii = true;
    let chunks_exact = slice.array_chunks::<32>();
    for &byte in chunks_exact.remainder() {
        if byte & 0b10000000 != 0 {
            is_plain_ascii = false;
        }
    }
    let mask = u8x32::splat(0b10000000);
    let zero = u8x32::splat(0);
    for &chunk in chunks_exact {
        let simd = u8x32::from_array(chunk);
        let xor = simd & mask;
        if xor != zero {
            is_plain_ascii = false;
        }
    }

    is_plain_ascii
}

impl Mutf8Str {
    pub fn to_string_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.slice)
    }

    #[inline]
    pub fn from_slice(slice: &[u8]) -> &Mutf8Str {
        // SAFETY: &[u8] and &Mutf8Str are the same layout.
        unsafe { mem::transmute::<&[u8], &Mutf8Str>(slice) }
    }

    // we can't implement FromStr on Cow<Mutf8Str>
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Cow<Mutf8Str> {
        match mutf8::encode(s) {
            Cow::Borrowed(b) => Cow::Borrowed(Mutf8Str::from_slice(b)),
            Cow::Owned(o) => Cow::Owned(Mutf8String { vec: o }),
        }
    }

    pub fn to_str(&self) -> Cow<str> {
        // fast check to skip if none of the bytes have the top bit set or are null
        if is_plain_ascii(&self.slice) {
            // SAFETY: &[u8] and &str are the same layout.
            unsafe { Cow::Borrowed(std::str::from_utf8_unchecked(&self.slice)) }
        } else {
            match mutf8::decode(&self.slice).expect("Mutf8Str must alwaus be valid MUTF-8") {
                Cow::Borrowed(b) => Cow::Borrowed(b),
                Cow::Owned(o) => Cow::Owned(o),
            }
        }
    }
}

impl fmt::Display for Mutf8Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_str())
    }
}

impl ToOwned for Mutf8Str {
    type Owned = Mutf8String;

    fn to_owned(&self) -> Self::Owned {
        Mutf8String {
            vec: self.slice.to_vec(),
        }
    }
}
impl Borrow<Mutf8Str> for Mutf8String {
    fn borrow(&self) -> &Mutf8Str {
        self.as_str()
    }
}

impl Mutf8String {
    #[inline]
    pub fn as_str(&self) -> &Mutf8Str {
        Mutf8Str::from_slice(self.vec.as_slice())
    }

    pub fn into_string(self) -> String {
        if is_plain_ascii(&self.vec) {
            // SAFETY: &[u8] and &str are the same layout.
            unsafe { String::from_utf8_unchecked(self.vec) }
        } else {
            match mutf8::decode(&self.vec).expect("Mutf8Str must alwaus be valid MUTF-8") {
                Cow::Borrowed(b) => b.to_owned(),
                Cow::Owned(o) => o,
            }
        }
    }
}
impl Deref for Mutf8String {
    type Target = Mutf8Str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

// TODO: make Mutf8 correct

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::mutf8::Mutf8Str;

    #[test]
    fn same_as_utf8() {
        let str = "Hello, world!";
        // 16-bit Unicode characters are the same in UTF-8 and MUTF-8:
        assert_eq!(
            Mutf8Str::from_str(str),
            Cow::Borrowed(Mutf8Str::from_slice(str.as_bytes()))
        );
        assert_eq!(Mutf8Str::from_str(str).to_str(), Cow::Borrowed(str));
    }

    #[test]
    fn surrogate_pairs() {
        let str = "\u{10401}";
        let mutf8_data = &[0xED, 0xA0, 0x81, 0xED, 0xB0, 0x81];
        // 'mutf8_data' is a byte slice containing a 6-byte surrogate pair which
        // becomes a 4-byte UTF-8 character.
        assert_eq!(
            Mutf8Str::from_slice(mutf8_data).to_str(),
            Cow::Borrowed(str)
        );
    }

    #[test]
    fn null_bytes() {
        let str = "\0";
        let mutf8_data = vec![0xC0, 0x80];
        // 'str' is a null character which becomes a two-byte MUTF-8 representation.
        assert_eq!(
            Mutf8Str::from_slice(&mutf8_data).to_str(),
            Cow::Borrowed(str)
        );
    }
}
