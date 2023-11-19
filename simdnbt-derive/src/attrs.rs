use syn::parse::{Parse, ParseStream};

#[derive(Default, Debug)]
pub struct FieldAttrs {
    pub rename: Option<String>,
}

impl Parse for FieldAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = Self::default();

        while !input.is_empty() {
            let attr = input.parse::<proc_macro2::Ident>()?;
            match attr.to_string().as_str() {
                "rename" => {
                    input.parse::<syn::Token![=]>()?;
                    let rename = input.parse::<syn::LitStr>()?;

                    attrs.rename = Some(rename.value());
                }
                _ => todo!(),
            }
        }

        Ok(attrs)
    }
}

pub fn parse_field_attrs(attrs: &[syn::Attribute]) -> FieldAttrs {
    let mut field_attrs = FieldAttrs::default();

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("simdnbt")) {
        let new_attr = attr
            .parse_args::<FieldAttrs>()
            .expect("invalid simdnbt attr");
        if let Some(rename) = new_attr.rename {
            field_attrs.rename = Some(rename);
        }
    }

    field_attrs
}
