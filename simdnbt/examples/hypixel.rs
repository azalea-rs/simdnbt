use std::{collections::HashMap, hint::black_box, io::Cursor};

use simdnbt::{owned::Nbt, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Item {
    pub id: i16,
    #[simdnbt(rename = "Damage")]
    pub damage: i16,
    #[simdnbt(rename = "Count")]
    pub count: i8,

    pub tag: ItemTag,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ItemTag {
    #[simdnbt(rename = "SkullOwner")]
    pub skull_owner: Option<SkullOwner>,
    #[simdnbt(rename = "ExtraAttributes")]
    pub extra_attributes: ExtraAttributes,
    pub display: ItemDisplay,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ExtraAttributes {
    pub id: Option<String>,
    pub modifier: Option<String>,

    pub ench: Option<simdnbt::owned::NbtCompound>,
    pub enchantments: Option<HashMap<String, i32>>,
    pub timestamp: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SkullOwner {
    pub properties: Properties,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Properties {
    pub textures: Vec<Texture>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Texture {
    #[simdnbt(rename = "Value")]
    pub value: String,
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
pub struct Base {
    #[simdnbt(rename = "i")]
    pub items: Vec<Option<Item>>,
}

fn main() {
    let input = black_box(include_bytes!("../tests/realworld.nbt"));

    for _ in 0..1 {
        let nbt = Nbt::read(&mut Cursor::new(input));
        let nbt = black_box(nbt.unwrap().unwrap());

        let data = Base::from_nbt(nbt).unwrap();

        // roundtrip
        let new_data = Base::from_nbt(data.clone().to_nbt()).unwrap();
        assert_eq!(data, new_data);

        println!("data: {:?}", data.items);
    }
}
