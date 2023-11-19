use std::{collections::HashMap, hint::black_box, io::Cursor};

use simdnbt::{owned::Nbt, Deserialize};

#[derive(Deserialize, Debug)]
pub struct Item {
    pub id: i16,
    #[simdnbt(rename = "Damage")]
    pub damage: i16,
    #[simdnbt(rename = "Count")]
    pub count: i8,

    pub tag: ItemTag,
}

#[derive(Deserialize, Debug)]
pub struct ItemTag {
    #[simdnbt(rename = "SkullOwner")]
    pub skull_owner: Option<SkullOwner>,
    #[simdnbt(rename = "ExtraAttributes")]
    pub extra_attributes: ExtraAttributes,
    pub display: ItemDisplay,
}

#[derive(Deserialize, Debug)]
pub struct ExtraAttributes {
    pub id: Option<String>,
    pub modifier: Option<String>,

    pub ench: Option<simdnbt::owned::NbtCompound>,
    pub enchantments: Option<HashMap<String, i32>>,
    pub timestamp: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct SkullOwner {
    pub properties: Properties,
}

#[derive(Deserialize, Debug)]
pub struct Properties {
    pub textures: Vec<Texture>,
}

#[derive(Deserialize, Debug)]
pub struct Texture {
    #[simdnbt(rename = "Value")]
    pub value: String,
}

#[derive(Deserialize, Debug)]
pub struct ItemDisplay {
    #[simdnbt(rename = "Name")]
    pub name: String,
    #[simdnbt(rename = "Lore")]
    pub lore: Vec<String>,

    pub color: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub struct Base {
    #[simdnbt(rename = "i")]
    pub items: Vec<Option<Item>>,
}

fn main() {
    let input = black_box(include_bytes!("../tests/realworld.nbt"));

    for _ in 0..1 {
        let nbt = Nbt::read(&mut Cursor::new(input));
        let nbt = black_box(nbt.unwrap().unwrap());

        let data = Base::from_nbt(nbt).unwrap().items;

        println!("data: {data:?}");
    }
}
