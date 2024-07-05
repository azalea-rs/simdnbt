#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(r) = simdnbt::borrow::read(&mut std::io::Cursor::new(data)) {
        if let simdnbt::borrow::Nbt::Some(r) = r {
            r.as_compound().to_owned();
        }
    }
});
