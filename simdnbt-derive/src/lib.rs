mod attrs;

use attrs::parse_field_attrs;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Deserialize, attributes(simdnbt))]
pub fn deserialize_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let mut field_deserializers = Vec::<proc_macro2::TokenStream>::new();

    match input.data {
        syn::Data::Struct(syn::DataStruct { fields, .. }) => match fields {
            syn::Fields::Named(syn::FieldsNamed { named, .. }) => {
                for field in named {
                    let struct_field_name = field.ident.unwrap();

                    let mut field_attrs = parse_field_attrs(&field.attrs);

                    let field_name = field_attrs
                        .rename
                        .take()
                        .unwrap_or_else(|| struct_field_name.to_string());

                    field_deserializers.push(quote! {
                        #struct_field_name: simdnbt::FromNbtTag::from_optional_nbt_tag(
                            nbt.take(#field_name)
                        )?.ok_or(simdnbt::DeserializeError::MismatchedFieldType)?
                    });
                }
            }
            syn::Fields::Unnamed(_) => todo!(),
            syn::Fields::Unit => todo!(),
        },
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    }

    let output = quote! {
        impl simdnbt::Deserialize for #ident {
            fn from_compound(mut nbt: simdnbt::owned::NbtCompound) -> Result<Self, simdnbt::DeserializeError> {
                Ok(Self {
                    #(#field_deserializers),*
                })
            }
        }
    };

    output.into()
}
