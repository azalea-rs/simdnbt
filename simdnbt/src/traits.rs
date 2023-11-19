use std::collections::HashMap;

use crate::DeserializeError;

pub trait Deserialize: Sized {
    fn from_nbt(nbt: crate::owned::BaseNbt) -> Result<Self, DeserializeError> {
        Self::from_compound(nbt.into_inner())
    }

    fn from_compound(compound: crate::owned::NbtCompound) -> Result<Self, DeserializeError>;
}

pub trait Serialize: Sized {
    fn to_nbt(self) -> crate::owned::BaseNbt {
        crate::owned::BaseNbt::new("", self.to_compound())
    }

    fn to_compound(self) -> crate::owned::NbtCompound;
}

pub trait FromNbtTag: Sized {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self>;
    fn from_optional_nbt_tag(
        tag: Option<crate::owned::NbtTag>,
    ) -> Result<Option<Self>, DeserializeError> {
        match tag {
            Some(tag) => Ok(Self::from_nbt_tag(tag)),
            None => Err(DeserializeError::MissingField),
        }
    }
}

pub trait ToNbtTag: Sized {
    fn to_nbt_tag(self) -> crate::owned::NbtTag;
    fn to_optional_nbt_tag(self) -> Option<crate::owned::NbtTag> {
        Some(self.to_nbt_tag())
    }
}

impl<T: FromNbtTag> Deserialize for HashMap<String, T> {
    fn from_compound(compound: crate::owned::NbtCompound) -> Result<Self, DeserializeError> {
        let mut hashmap = HashMap::with_capacity(compound.values.len());

        for (k, v) in compound.values {
            hashmap.insert(
                k.to_string(),
                T::from_nbt_tag(v).ok_or(DeserializeError::MismatchedFieldType)?,
            );
        }

        Ok(hashmap)
    }
}
impl<T: ToNbtTag> Serialize for HashMap<String, T> {
    fn to_compound(self) -> crate::owned::NbtCompound {
        let mut compound = crate::owned::NbtCompound::new();

        for (k, v) in self {
            compound.insert(k, v.to_nbt_tag());
        }

        compound
    }
}

impl Deserialize for crate::owned::NbtCompound {
    fn from_compound(compound: crate::owned::NbtCompound) -> Result<Self, DeserializeError> {
        Ok(compound)
    }
}
impl Serialize for crate::owned::NbtCompound {
    fn to_compound(self) -> crate::owned::NbtCompound {
        self
    }
}

impl<T: Deserialize> FromNbtTag for T {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.into_compound()
            .and_then(|c| Self::from_compound(c).ok())
    }
}

impl<T: Serialize> ToNbtTag for T {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::Compound(self.to_compound())
    }
}

// standard nbt types
impl FromNbtTag for i8 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.byte()
    }
}
impl ToNbtTag for i8 {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::Byte(self)
    }
}

impl FromNbtTag for i16 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.short()
    }
}
impl ToNbtTag for i16 {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::Short(self)
    }
}

impl FromNbtTag for i32 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.int()
    }
}
impl ToNbtTag for i32 {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::Int(self)
    }
}

impl FromNbtTag for i64 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.long()
    }
}
impl ToNbtTag for i64 {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::Long(self)
    }
}

impl FromNbtTag for f32 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.float()
    }
}
impl ToNbtTag for f32 {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::Float(self)
    }
}

impl FromNbtTag for f64 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.double()
    }
}
impl ToNbtTag for f64 {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::Double(self)
    }
}

impl FromNbtTag for String {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.string().map(|s| s.to_string())
    }
}
impl ToNbtTag for String {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::String(self.into())
    }
}

// lists
impl FromNbtTag for Vec<String> {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.list().and_then(|l| {
            l.strings()
                .map(|s| s.iter().map(|s| s.to_string()).collect())
        })
    }
}
impl ToNbtTag for Vec<String> {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::List(crate::owned::NbtList::String(
            self.into_iter().map(|s| s.into()).collect(),
        ))
    }
}

// slightly less standard types
impl<T: FromNbtTag> FromNbtTag for Option<T> {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        Some(T::from_nbt_tag(tag))
    }
    fn from_optional_nbt_tag(
        tag: Option<crate::owned::NbtTag>,
    ) -> Result<Option<Self>, DeserializeError> {
        match tag {
            Some(tag) => Ok(Some(T::from_nbt_tag(tag))),
            None => Ok(Some(None)),
        }
    }
}
impl<T: ToNbtTag> ToNbtTag for Option<T> {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        panic!("Called to_nbt_tag on Option<T>. Use to_optional_nbt_tag instead.")
    }
    fn to_optional_nbt_tag(self) -> Option<crate::owned::NbtTag> {
        match self {
            Some(t) => Some(t.to_nbt_tag()),
            None => None,
        }
    }
}

impl<T: Deserialize> FromNbtTag for Vec<Option<T>> {
    /// A list of compounds where `None` is an empty compound
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        let list = tag.into_list()?.into_compounds()?;
        let mut vec = Vec::with_capacity(list.len());
        for tag in list {
            if tag.values.is_empty() {
                vec.push(None);
            } else {
                vec.push(Some(T::from_compound(tag).ok()?));
            }
        }

        Some(vec)
    }
}
impl<T: Serialize> ToNbtTag for Vec<Option<T>> {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::List(crate::owned::NbtList::Compound(
            self.into_iter()
                .map(|t| match t {
                    Some(t) => t.to_compound(),
                    None => crate::owned::NbtCompound::new(),
                })
                .collect(),
        ))
    }
}

impl<T: Deserialize> FromNbtTag for Vec<T> {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        let list = tag.into_list()?.into_compounds()?;
        let mut vec = Vec::with_capacity(list.len());
        for tag in list {
            vec.push(T::from_compound(tag).ok()?);
        }

        Some(vec)
    }
}
impl<T: Serialize> ToNbtTag for Vec<T> {
    fn to_nbt_tag(self) -> crate::owned::NbtTag {
        crate::owned::NbtTag::List(crate::owned::NbtList::Compound(
            self.into_iter().map(|t| t.to_compound()).collect(),
        ))
    }
}

impl FromNbtTag for bool {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.byte().map(|b| b != 0)
    }
}