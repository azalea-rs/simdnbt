//! The string representation used in NBT.

use std::{
    borrow::{Borrow, Cow},
    fmt, mem,
    ops::Deref,
};

/// A M-UTF8 string slice.
#[derive(Debug, Eq, PartialEq)]
pub struct Mutf8Str {
    pub(crate) slice: [u8],
}
/// An owned M-UTF8 string.
#[derive(Debug, Eq, PartialEq)]
pub struct Mutf8String {
    vec: Vec<u8>,
}

impl Mutf8Str {
    pub fn to_string_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.slice)
    }

    #[inline]
    pub fn from_slice(slice: &[u8]) -> &Mutf8Str {
        // SAFETY: &[u8] and &Mutf8Str are the same layout.
        unsafe { mem::transmute(slice) }
    }

    /// Returns whether the given byte slice is a valid M-UTF-8 string.
    pub fn is_valid(_slice: &[u8]) -> bool {
        true
    }

    pub fn from_str<'a>(s: &'a str) -> Cow<'a, Mutf8Str> {
        match mutf8::encode(s) {
            Cow::Borrowed(b) => Cow::Borrowed(Mutf8Str::from_slice(b)),
            Cow::Owned(o) => Cow::Owned(Mutf8String { vec: o }),
        }
    }
    pub fn to_str(&self) -> Cow<str> {
        match mutf8::decode(&self.slice).expect("Mutf8Str must alwaus be valid MUTF-8") {
            Cow::Borrowed(b) => Cow::Borrowed(b),
            Cow::Owned(o) => Cow::Owned(o),
        }
    }
}

impl fmt::Display for Mutf8Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let self_as_str = self.to_str();
        self_as_str.fmt(f)
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
