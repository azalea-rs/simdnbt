use std::{any, borrow::Cow, collections::HashMap, hash::Hash, io::Cursor};

use byteorder::ReadBytesExt;

use crate::{
    common::{self, read_string},
    mutf8::Mutf8String,
    owned,
    reader::{Reader, ReaderFromCursor},
    DeserializeError, Error, Mutf8Str,
};

pub trait Deserialize<'a>: Sized {
    /// The NBT type that we expect the data to be. If this is set to END_ID,
    /// then any type is allowed.
    const NBT_TYPE_ID: u8;
    type Partial<'b>;

    fn read(data: &mut Cursor<&'a [u8]>) -> Result<(&'a Mutf8Str, Self), DeserializeError> {
        let root_type = data.read_u8().map_err(|_| Error::UnexpectedEof)?;
        if root_type != common::COMPOUND_ID {
            return Err(Error::InvalidRootType(root_type).into());
        }

        let mut data = ReaderFromCursor::new(data);
        let name = read_string(&mut data)?;

        Ok((
            name,
            Self::read_value_direct_with_explicit_type(&mut data, root_type)?,
        ))
    }

    fn read_value_direct(data: &mut Reader<'a>) -> Result<Self, DeserializeError>;

    #[inline]
    fn read_value_direct_with_explicit_type(
        data: &mut Reader<'a>,
        type_id: u8,
    ) -> Result<Self, DeserializeError> {
        debug_assert_eq!(type_id, Self::NBT_TYPE_ID);

        Self::read_value_direct(data)
    }

    #[inline]
    fn type_matches(type_id: u8) -> bool {
        Self::NBT_TYPE_ID == common::END_ID || type_id == Self::NBT_TYPE_ID
    }

    #[inline]
    fn update_partial(
        _partial: &mut Self::Partial<'_>,
        _name: &Mutf8Str,
        _tag_type: u8,
        _data: &mut Reader<'a>,
    ) -> Result<bool, DeserializeError> {
        panic!("{} cannot be flattened", any::type_name::<Self>())
    }

    #[inline]
    fn from_partial(_partial: Self::Partial<'_>) -> Result<Self, DeserializeError> {
        unimplemented!()
    }

    #[inline]
    fn try_flatten_with_option(other: Option<Self>) -> Option<Self> {
        // can't unflatten by default
        other
    }
}

pub trait Serialize: Sized {
    fn to_nbt(self) -> owned::BaseNbt {
        owned::BaseNbt::new("", self.to_compound())
    }

    fn to_compound(self) -> owned::NbtCompound;
}

pub trait ToNbtTag: Sized {
    fn to_nbt_tag(self) -> owned::NbtTag;
    fn to_optional_nbt_tag(self) -> Option<owned::NbtTag> {
        Some(self.to_nbt_tag())
    }
}

impl<'a, K: From<&'a Mutf8Str> + Eq + Hash, V: Deserialize<'a>> Deserialize<'a> for HashMap<K, V> {
    const NBT_TYPE_ID: u8 = common::COMPOUND_ID;
    type Partial<'b> = HashMap<K, V>;

    fn read_value_direct(data: &mut Reader<'a>) -> Result<Self, DeserializeError> {
        let mut map = HashMap::new();

        loop {
            let tag_type = data.read_u8()?;
            if tag_type == common::END_ID {
                break;
            }
            let name = read_string(data)?;

            if !V::type_matches(tag_type) {
                return Err(DeserializeError::MismatchedFieldType("HashMap"));
            }
            let value = V::read_value_direct_with_explicit_type(data, tag_type)?;

            map.insert(name.into(), value);
        }

        Ok(map)
    }
}
impl<'a, K: From<&'a Mutf8Str> + Eq + Hash, V: Deserialize<'a>> Deserialize<'a> for Vec<(K, V)> {
    const NBT_TYPE_ID: u8 = common::COMPOUND_ID;
    type Partial<'b> = HashMap<K, V>;

    fn read_value_direct(data: &mut Reader<'a>) -> Result<Self, DeserializeError> {
        let mut map = Vec::new();

        loop {
            let tag_type = data.read_u8()?;
            if tag_type == common::END_ID {
                break;
            }
            let name = read_string(data)?;

            if !V::type_matches(tag_type) {
                return Err(DeserializeError::MismatchedFieldType("HashMap"));
            }
            let value = V::read_value_direct_with_explicit_type(data, tag_type)?;

            map.push((name.into(), value));
        }

        Ok(map)
    }
}
impl<K: Into<Mutf8String> + Eq + Hash, V: ToNbtTag> Serialize for HashMap<K, V> {
    fn to_compound(self) -> owned::NbtCompound {
        let mut compound = owned::NbtCompound::new();

        for (k, v) in self {
            compound.insert(k, v.to_nbt_tag());
        }

        compound
    }
}

impl Deserialize<'_> for owned::NbtCompound {
    const NBT_TYPE_ID: u8 = common::COMPOUND_ID;

    type Partial<'b> = owned::NbtCompound;

    fn read_value_direct(data: &mut Reader<'_>) -> Result<Self, DeserializeError> {
        owned::NbtCompound::read(data).map_err(Into::into)
    }

    fn update_partial(
        partial: &mut Self::Partial<'_>,
        name: &Mutf8Str,
        tag_type: u8,
        data: &mut Reader<'_>,
    ) -> Result<bool, DeserializeError> {
        let tag = owned::NbtTag::read_with_type(data, tag_type, 0).map_err(Error::from)?;
        partial.insert(name.to_owned(), tag);
        Ok(true)
    }

    fn from_partial(partial: Self::Partial<'_>) -> Result<Self, DeserializeError> {
        Ok(partial)
    }
}
impl Serialize for owned::NbtCompound {
    fn to_compound(self) -> owned::NbtCompound {
        self
    }
}

impl<T: Serialize> ToNbtTag for T {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Compound(self.to_compound())
    }
}

impl Deserialize<'_> for owned::NbtTag {
    const NBT_TYPE_ID: u8 = common::END_ID;

    type Partial<'b> = ();

    fn read_value_direct(_data: &mut Reader<'_>) -> Result<Self, DeserializeError> {
        unimplemented!("can't deserialize an NbtTag without the type being known")
    }

    fn read_value_direct_with_explicit_type(
        data: &mut Reader<'_>,
        type_id: u8,
    ) -> Result<Self, DeserializeError> {
        let tag = owned::NbtTag::read_with_type(data, type_id, 0).map_err(Error::from)?;
        Ok(tag)
    }
}
impl ToNbtTag for owned::NbtTag {
    fn to_nbt_tag(self) -> owned::NbtTag {
        self
    }
}

// // standard nbt types
impl Deserialize<'_> for i8 {
    const NBT_TYPE_ID: u8 = common::BYTE_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        data.read_i8().map_err(Into::into)
    }
}
impl ToNbtTag for i8 {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Byte(self)
    }
}

impl Deserialize<'_> for i16 {
    const NBT_TYPE_ID: u8 = common::SHORT_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        data.read_i16().map_err(Into::into)
    }
}
impl ToNbtTag for i16 {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Short(self)
    }
}

impl Deserialize<'_> for i32 {
    const NBT_TYPE_ID: u8 = common::INT_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        data.read_i32().map_err(Into::into)
    }
}
impl ToNbtTag for i32 {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Int(self)
    }
}

impl Deserialize<'_> for i64 {
    const NBT_TYPE_ID: u8 = common::LONG_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        data.read_i64().map_err(Into::into)
    }
}
impl ToNbtTag for i64 {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Long(self)
    }
}

impl Deserialize<'_> for f32 {
    const NBT_TYPE_ID: u8 = common::FLOAT_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        data.read_f32().map_err(Into::into)
    }
}
impl ToNbtTag for f32 {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Float(self)
    }
}

impl Deserialize<'_> for f64 {
    const NBT_TYPE_ID: u8 = common::DOUBLE_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        data.read_f64().map_err(Into::into)
    }
}
impl ToNbtTag for f64 {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Double(self)
    }
}

impl Deserialize<'_> for String {
    const NBT_TYPE_ID: u8 = common::STRING_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        let str = common::read_string(data)?;
        Ok(str.to_string())
    }
}
impl<'a> Deserialize<'a> for &'a Mutf8Str {
    const NBT_TYPE_ID: u8 = common::STRING_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader<'a>) -> Result<Self, DeserializeError> {
        common::read_string(data).map_err(Into::into)
    }
}
impl<'a> Deserialize<'a> for Cow<'a, str> {
    const NBT_TYPE_ID: u8 = common::STRING_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader<'a>) -> Result<Self, DeserializeError> {
        common::read_string(data)
            .map_err(Into::into)
            .map(|s| s.to_str())
    }
}
impl ToNbtTag for Mutf8String {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::String(self)
    }
}
impl ToNbtTag for &Mutf8Str {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::String(self.to_owned())
    }
}
impl ToNbtTag for Cow<'_, str> {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::String(self.into_owned().into())
    }
}

impl ToNbtTag for String {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::String(self.into())
    }
}

impl ToNbtTag for &str {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::String(self.into())
    }
}

// unsigned integers
impl Deserialize<'_> for u8 {
    const NBT_TYPE_ID: u8 = common::BYTE_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        data.read_u8().map_err(Into::into)
    }
}
impl ToNbtTag for u8 {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Byte(self as i8)
    }
}

impl Deserialize<'_> for u16 {
    const NBT_TYPE_ID: u8 = common::SHORT_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        data.read_u16().map_err(Into::into)
    }
}
impl ToNbtTag for u16 {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Short(self as i16)
    }
}

impl Deserialize<'_> for u32 {
    const NBT_TYPE_ID: u8 = common::INT_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        data.read_u32().map_err(Into::into)
    }
}
impl ToNbtTag for u32 {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Int(self as i32)
    }
}

impl Deserialize<'_> for u64 {
    const NBT_TYPE_ID: u8 = common::LONG_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader) -> Result<Self, DeserializeError> {
        data.read_u64().map_err(Into::into)
    }
}
impl ToNbtTag for u64 {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Long(self as i64)
    }
}

// lists
impl ToNbtTag for Vec<String> {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::List(owned::NbtList::String(
            self.into_iter().map(|s| s.into()).collect(),
        ))
    }
}

// slightly less standard types
impl<'a, T: Deserialize<'a>> Deserialize<'a> for Option<T> {
    const NBT_TYPE_ID: u8 = T::NBT_TYPE_ID;
    type Partial<'b> = Option<T>;

    fn read_value_direct(data: &mut Reader<'a>) -> Result<Self, DeserializeError> {
        // empty compounds also count as None
        if Self::NBT_TYPE_ID == common::COMPOUND_ID {
            let next_tag_type = data.peek_u8()?;
            if next_tag_type == common::END_ID {
                data.skip(1)?;
                return Ok(None);
            }
        }

        Ok(Some(T::read_value_direct(data)?))
    }

    fn try_flatten_with_option(other: Option<Self>) -> Option<Self> {
        Some(other.flatten())
    }
}
impl<T: ToNbtTag> ToNbtTag for Option<T> {
    fn to_nbt_tag(self) -> owned::NbtTag {
        panic!("Called to_nbt_tag on Option<T>. Use to_optional_nbt_tag instead.")
    }
    fn to_optional_nbt_tag(self) -> Option<owned::NbtTag> {
        self.map(|t| t.to_nbt_tag())
    }
}

impl<T: Serialize> ToNbtTag for Vec<Option<T>> {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::List(owned::NbtList::Compound(
            self.into_iter()
                .map(|t| match t {
                    Some(t) => t.to_compound(),
                    None => owned::NbtCompound::new(),
                })
                .collect(),
        ))
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Vec<T> {
    const NBT_TYPE_ID: u8 = common::LIST_ID;
    type Partial<'b> = ();

    fn read_value_direct(data: &mut Reader<'a>) -> Result<Self, DeserializeError> {
        let tag_type = data.read_u8()?;
        let list_length = data.read_i32()?;
        if tag_type == common::END_ID || list_length <= 0 {
            return Ok(Vec::new());
        }
        if !T::type_matches(tag_type) {
            return Err(DeserializeError::MismatchedListType(tag_type));
        }

        let mut vec = Vec::with_capacity(list_length.min(128) as usize);
        for _ in 0..list_length {
            vec.push(T::read_value_direct_with_explicit_type(data, tag_type)?);
        }

        Ok(vec)
    }
}
impl<T: Serialize> ToNbtTag for Vec<T> {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::List(owned::NbtList::Compound(
            self.into_iter().map(|t| t.to_compound()).collect(),
        ))
    }
}

impl ToNbtTag for bool {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::Byte(if self { 1 } else { 0 })
    }
}

impl ToNbtTag for owned::NbtList {
    fn to_nbt_tag(self) -> owned::NbtTag {
        owned::NbtTag::List(self)
    }
}
