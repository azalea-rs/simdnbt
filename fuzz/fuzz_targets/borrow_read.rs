#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(simdnbt::borrow::Nbt::Some(r)) = simdnbt::borrow::read(&mut Cursor::new(data)) {
        r.as_compound().to_owned();
    }
});
