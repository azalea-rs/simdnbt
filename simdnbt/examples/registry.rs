use std::{collections::HashMap, io::Cursor};

use simdnbt::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[simdnbt(deny_unknown_fields)]
pub struct TrimMaterialElement {
    pub asset_name: String,
    pub item_model_index: f32,
    pub override_armor_materials: HashMap<String, String>,
    pub description: Option<String>,
}

fn main() {
    let original = TrimMaterialElement {
        asset_name: "asset name".to_string(),
        item_model_index: 0.0,
        override_armor_materials: HashMap::from_iter(vec![
            ("asdf".into(), "fdsa".into()),
            ("dsfgdgh".into(), "fgjrtiu".into()),
        ]),
        description: Some("description".to_string()),
    };

    let nbt = original.clone().to_nbt();
    let mut buf = Vec::new();
    nbt.write(&mut buf);

    let (_, rewritten) = TrimMaterialElement::read(&mut Cursor::new(&buf)).unwrap();

    assert_eq!(original, rewritten);
}
