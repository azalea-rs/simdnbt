use std::collections::HashMap;

use crate::DeserializeError;

pub trait Deserialize: Sized {
    fn from_nbt(nbt: crate::owned::BaseNbt) -> Result<Self, DeserializeError> {
        Self::from_compound(nbt.into_inner())
    }

    fn from_compound(compound: crate::owned::NbtCompound) -> Result<Self, DeserializeError>;
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

impl Deserialize for crate::owned::NbtCompound {
    fn from_compound(compound: crate::owned::NbtCompound) -> Result<Self, DeserializeError> {
        Ok(compound)
    }
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

impl<T: Deserialize> FromNbtTag for T {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.into_compound()
            .and_then(|c| Self::from_compound(c).ok())
    }
}

// standard nbt types
impl FromNbtTag for i8 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.byte()
    }
}
impl FromNbtTag for i16 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.short()
    }
}
impl FromNbtTag for i32 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.int()
    }
}
impl FromNbtTag for i64 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.long()
    }
}
impl FromNbtTag for f32 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.float()
    }
}
impl FromNbtTag for f64 {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.double()
    }
}
impl FromNbtTag for String {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.string().map(|s| s.to_string())
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

impl FromNbtTag for bool {
    fn from_nbt_tag(tag: crate::owned::NbtTag) -> Option<Self> {
        tag.byte().map(|b| b != 0)
    }
}
