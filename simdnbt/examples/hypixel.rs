use std::{borrow::Cow, collections::HashMap, hint::black_box, io::Cursor};

use simdnbt::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Item<'a> {
    pub id: i16,
    #[simdnbt(rename = "Damage")]
    pub damage: Option<i16>,
    #[simdnbt(rename = "Count")]
    pub count: i8,

    pub tag: ItemTag<'a>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ItemTag<'a> {
    #[simdnbt(rename = "SkullOwner")]
    pub skull_owner: Option<SkullOwner<'a>>,
    #[simdnbt(rename = "ExtraAttributes")]
    pub extra_attributes: Option<ExtraAttributes<'a>>,
    pub display: Option<ItemDisplay>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ExtraAttributes<'a> {
    pub id: Option<Cow<'a, str>>,
    pub modifier: Option<Cow<'a, str>>,

    pub ench: Option<simdnbt::owned::NbtCompound>,
    pub enchantments: Option<HashMap<String, i32>>,
    pub timestamp: Option<Cow<'a, str>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SkullOwner<'a> {
    #[simdnbt(rename = "Properties")]
    pub properties: Properties<'a>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Properties<'a> {
    pub textures: Vec<Texture<'a>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Texture<'a> {
    #[simdnbt(rename = "Value")]
    pub value: Cow<'a, str>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ItemDisplay {
    #[simdnbt(rename = "Name")]
    pub name: String,
    #[simdnbt(rename = "Lore")]
    pub lore: Vec<String>,

    pub color: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Base<'a> {
    #[simdnbt(rename = "i")]
    pub items: Vec<Option<Item<'a>>>,
}

fn main() {
    let input = black_box(include_bytes!("../tests/hypixel.nbt"));

    for _ in 0..1 {
        let (_, data) = Base::read(&mut Cursor::new(input)).unwrap();

        // roundtrip
        let mut new_nbt_bytes = Vec::new();
        data.clone().to_nbt().write(&mut new_nbt_bytes);

        let (_, new_data) = Base::read(&mut Cursor::new(&new_nbt_bytes)).unwrap();
        assert_eq!(data, new_data);
    }
}
