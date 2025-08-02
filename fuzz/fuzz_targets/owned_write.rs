#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(simdnbt::borrow::Nbt::Some(r)) =
        simdnbt::borrow::read(&mut std::io::Cursor::new(data))
    {
        simdnbt::owned::BaseNbt::write_unnamed(&r.as_compound().to_owned().into(), &mut Vec::new());
    }
});
