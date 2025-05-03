//! Validate NBT without actually reading it.
//!
//! This is used by `simdnbt-derive` for skipping unused fields.

mod compound;
mod list;

use std::io::Cursor;

use byteorder::ReadBytesExt;
use compound::ParsingStackElementKind;

pub use self::{compound::NbtCompound, list::NbtList};
use self::{
    compound::{read_tag_in_compound, ParsingStack},
    list::{read_compound_in_list, read_list_in_list},
};
use crate::{
    common::{read_string, COMPOUND_ID, END_ID},
    reader::{Reader, ReaderFromCursor},
    Error,
};

/// Read a normal root NBT compound. This is either empty or has a name and
/// compound tag.
pub fn read(data: &mut Cursor<&[u8]>) -> Result<(), Error> {
    let root_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
    if root_type == END_ID {
        return Ok(());
    }
    if root_type != COMPOUND_ID {
        return Err(Error::InvalidRootType(root_type));
    }
    // our Reader type is faster than Cursor
    let mut data = ReaderFromCursor::new(data);
    read_string(&mut data)?;

    let mut stack = ParsingStack::new();
    stack.push(ParsingStackElementKind::Compound)?;

    read_with_stack(&mut data, &mut stack)?;

    Ok(())
}
/// Read a root NBT compound, but without reading the name. This is used in
/// Minecraft when reading NBT over the network.
///
/// This is similar to [`read_tag`], but only allows the data to be empty or a
/// compound.
pub fn read_unnamed(data: &mut Cursor<&[u8]>) -> Result<(), Error> {
    let root_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
    if root_type == END_ID {
        return Ok(());
    }
    if root_type != COMPOUND_ID {
        return Err(Error::InvalidRootType(root_type));
    }
    read_compound(data)?;
    Ok(())
}
/// Read a compound tag. This may have any number of items.
pub fn read_compound(data: &mut Cursor<&[u8]>) -> Result<(), Error> {
    let mut stack = ParsingStack::new();
    let mut data = ReaderFromCursor::new(data);
    stack.push(ParsingStackElementKind::Compound)?;
    read_with_stack(&mut data, &mut stack)?;
    Ok(())
}
/// Read an NBT tag, without reading its name. This may be any type of tag
/// except for an end tag. If you need to be able to handle end tags, use
/// [`read_optional_tag`].
pub fn read_tag(data: &mut Cursor<&[u8]>) -> Result<(), Error> {
    let mut stack = ParsingStack::new();

    let mut data = ReaderFromCursor::new(data);

    let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
    if tag_type == END_ID {
        return Err(Error::InvalidRootType(0));
    }
    compound::read_tag(&mut data, &mut stack, tag_type)?;
    read_with_stack(&mut data, &mut stack)?;

    Ok(())
}

#[doc(hidden)]
pub fn internal_read_tag(data: &mut Reader, tag_type: u8) -> Result<(), Error> {
    let mut stack = ParsingStack::new();
    compound::read_tag(data, &mut stack, tag_type)?;
    read_with_stack(data, &mut stack)?;
    Ok(())
}

/// Read any NBT tag, without reading its name. This may be any type of tag,
/// including an end tag.
pub fn read_optional_tag(data: &mut Cursor<&[u8]>) -> Result<(), Error> {
    let mut stack = ParsingStack::new();

    let mut data = ReaderFromCursor::new(data);

    let tag_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
    if tag_type == END_ID {
        return Ok(());
    }
    compound::read_tag(&mut data, &mut stack, tag_type)?;
    read_with_stack(&mut data, &mut stack)?;

    Ok(())
}

fn read_with_stack<'a>(data: &mut Reader<'a>, stack: &mut ParsingStack) -> Result<(), Error> {
    while !stack.is_empty() {
        let top = stack.peek();
        match top {
            ParsingStackElementKind::Compound => read_tag_in_compound(data, stack)?,
            ParsingStackElementKind::ListOfCompounds => read_compound_in_list(data, stack)?,
            ParsingStackElementKind::ListOfLists => read_list_in_list(data, stack)?,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use byteorder::{WriteBytesExt, BE};
    use flate2::read::GzDecoder;

    use super::*;
    use crate::common::{INT_ID, LIST_ID, LONG_ID};

    #[test]
    fn hello_world() {
        super::read(&mut Cursor::new(include_bytes!(
            "../../tests/hello_world.nbt"
        )))
        .unwrap();
    }

    #[test]
    fn simple_player() {
        let src = include_bytes!("../../tests/simple_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        super::read(&mut Cursor::new(&decoded_src)).unwrap();
    }

    #[test]
    fn read_complex_player() {
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        super::read(&mut Cursor::new(&decoded_src)).unwrap();
    }

    #[test]
    fn read_hypixel() {
        let src = include_bytes!("../../tests/hypixel.nbt").to_vec();
        super::read(&mut Cursor::new(&src[..])).unwrap();
    }

    #[test]
    fn inttest_1023() {
        super::read(&mut Cursor::new(include_bytes!(
            "../../tests/inttest1023.nbt"
        )))
        .unwrap();
    }

    #[test]
    fn inttest_1024() {
        let mut data = Vec::new();
        data.write_u8(COMPOUND_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(LIST_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(INT_ID).unwrap();
        data.write_i32::<BE>(1024).unwrap();
        for i in 0..1024 {
            data.write_i32::<BE>(i).unwrap();
        }
        data.write_u8(END_ID).unwrap();

        super::read(&mut Cursor::new(&data)).unwrap();
    }

    #[test]
    fn inttest_1021() {
        let mut data = Vec::new();
        data.write_u8(COMPOUND_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(LIST_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(INT_ID).unwrap();
        data.write_i32::<BE>(1021).unwrap();
        for i in 0..1021 {
            data.write_i32::<BE>(i).unwrap();
        }
        data.write_u8(END_ID).unwrap();

        super::read(&mut Cursor::new(&data)).unwrap();
    }

    #[test]
    fn longtest_1023() {
        let mut data = Vec::new();
        data.write_u8(COMPOUND_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(LIST_ID).unwrap();
        data.write_u16::<BE>(0).unwrap();
        data.write_u8(LONG_ID).unwrap();
        data.write_i32::<BE>(1023).unwrap();
        for i in 0..1023 {
            data.write_i64::<BE>(i).unwrap();
        }
        data.write_u8(END_ID).unwrap();

        super::read(&mut Cursor::new(&data)).unwrap();
    }

    #[test]
    fn compound_eof() {
        let mut data = Vec::new();
        data.write_u8(COMPOUND_ID).unwrap(); // root type
        data.write_u16::<BE>(0).unwrap(); // root name length
        data.write_u8(COMPOUND_ID).unwrap(); // first element type
        data.write_u16::<BE>(0).unwrap(); // first element name length
                                          // eof

        let res = super::read(&mut Cursor::new(&data));
        assert_eq!(res, Err(Error::UnexpectedEof));
    }

    #[test]
    fn read_complex_player_as_tag() {
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();

        let mut decoded_src_as_tag = Vec::new();
        decoded_src_as_tag.push(COMPOUND_ID);
        decoded_src_as_tag.extend_from_slice(&decoded_src);
        decoded_src_as_tag.push(END_ID);

        super::read_tag(&mut Cursor::new(&decoded_src_as_tag)).unwrap();
    }

    #[test]
    fn byte_array() {
        // found from fuzzing
        let data = [10, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0];
        super::read(&mut Cursor::new(&data)).unwrap();
    }
    #[test]
    fn list_of_empty_lists() {
        // found from fuzzing
        // BaseNbt { name: m"", tag: NbtTag::NbtCompound { m"":
        // NbtTag::List(List::List([List::Empty])) } }
        let data = [10, 0, 0, 9, 0, 0, 9, 0, 0, 0, 1, 0, 9, 0, 0, 0, 0];
        super::read(&mut Cursor::new(&data)).unwrap();
    }
    #[test]
    fn list_of_byte_arrays() {
        // BaseNbt { name: m"", tag: NbtCompound { values: [(m"",
        // List(List([List::ByteArray([])])))] } }
        let data = [10, 0, 0, 9, 0, 0, 9, 0, 0, 0, 1, 7, 0, 0, 0, 0, 0];
        super::read(&mut Cursor::new(&data)).unwrap();
    }

    #[test]
    fn compound_len() {
        let src = include_bytes!("../../tests/complex_player.dat").to_vec();
        let mut src_slice = src.as_slice();
        let mut decoded_src_decoder = GzDecoder::new(&mut src_slice);
        let mut decoded_src = Vec::new();
        decoded_src_decoder.read_to_end(&mut decoded_src).unwrap();
        super::read(&mut Cursor::new(&decoded_src)).unwrap();
    }
}
