use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::{
    Mutf8String,
    owned::{BaseNbt, Nbt, NbtCompound, NbtList, NbtTag},
};

impl Serialize for NbtCompound {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // compounds with an empty string as the key are serialized as their value
        // (minecraft uses this for non-homogeneous lists)
        if self.values.len() == 1
            && let Some(value) = self.get("")
        {
            return value.serialize(serializer);
        }

        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (key, value) in &self.values {
            map.serialize_entry(&key.as_str().to_str(), value)?;
        }
        map.end()
    }
}

impl Serialize for NbtTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            NbtTag::Byte(v) => v.serialize(serializer),
            NbtTag::Short(v) => v.serialize(serializer),
            NbtTag::Int(v) => v.serialize(serializer),
            NbtTag::Long(v) => v.serialize(serializer),
            NbtTag::Float(v) => v.serialize(serializer),
            NbtTag::Double(v) => v.serialize(serializer),
            NbtTag::ByteArray(v) => v.serialize(serializer),
            NbtTag::String(v) => v.serialize(serializer),
            NbtTag::List(v) => v.serialize(serializer),
            NbtTag::Compound(v) => v.serialize(serializer),
            NbtTag::IntArray(v) => v.serialize(serializer),
            NbtTag::LongArray(v) => v.serialize(serializer),
        }
    }
}
impl Serialize for NbtList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            NbtList::Byte(v) => v.serialize(serializer),
            NbtList::Short(v) => v.serialize(serializer),
            NbtList::Int(v) => v.serialize(serializer),
            NbtList::Long(v) => v.serialize(serializer),
            NbtList::Float(v) => v.serialize(serializer),
            NbtList::Double(v) => v.serialize(serializer),
            NbtList::ByteArray(v) => v.serialize(serializer),
            NbtList::String(v) => v.serialize(serializer),
            NbtList::Compound(v) => v.serialize(serializer),
            NbtList::IntArray(v) => v.serialize(serializer),
            NbtList::LongArray(v) => v.serialize(serializer),
            NbtList::Empty => ([] as [(); 0]).serialize(serializer),
            NbtList::List(v) => v.serialize(serializer),
        }
    }
}

impl Serialize for Mutf8String {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.as_str().to_str())
    }
}
impl Serialize for BaseNbt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.tag.serialize(serializer)
    }
}
impl Serialize for Nbt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Nbt::Some(base_nbt) => base_nbt.serialize(serializer),
            Nbt::None => serializer.serialize_none(),
        }
    }
}
