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

                    if field_attrs.flatten {
                        field_deserializers.push(quote! {
                            #struct_field_name: simdnbt::Deserialize::from_compound(nbt)?,
                        })
                    } else {
                        field_deserializers.push(quote! {
                            #struct_field_name: simdnbt::FromNbtTag::from_optional_nbt_tag(
                                nbt.take(#field_name)
                            )?.ok_or(simdnbt::DeserializeError::MismatchedFieldType)?
                        });
                    }
                }
            }
            syn::Fields::Unnamed(_) => todo!(),
            syn::Fields::Unit => todo!(),
        },
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    }

    let generics = input.generics;
    let struct_attrs = attrs::parse_struct_attrs(&input.attrs);

    let extra_checks = if struct_attrs.deny_unknown_fields {
        quote! {
            if !nbt.is_empty() {
                return Err(simdnbt::DeserializeError::UnknownField(nbt.keys().next().unwrap().clone()));
            }
        }
    } else {
        quote! {}
    };

    let output = quote! {
        impl #generics simdnbt::Deserialize for #ident #generics {
            fn from_compound(mut nbt: simdnbt::owned::NbtCompound) -> Result<Self, simdnbt::DeserializeError> {
                let value = Self {
                    #(#field_deserializers),*
                };
                #extra_checks
                Ok(value)
            }
        }
    };

    output.into()
}

#[proc_macro_derive(Serialize, attributes(simdnbt))]
pub fn serialize_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let mut field_serializers = Vec::<proc_macro2::TokenStream>::new();

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

                    field_serializers.push(quote! {
                        if let Some(item) = simdnbt::ToNbtTag::to_optional_nbt_tag(self.#struct_field_name) {
                            nbt.insert(#field_name, item);
                        }
                    });
                }
            }
            syn::Fields::Unnamed(_) => todo!(),
            syn::Fields::Unit => todo!(),
        },
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    }

    let generics = input.generics;
    let struct_attrs = attrs::parse_struct_attrs(&input.attrs);

    let output = quote! {
        impl #generics simdnbt::Serialize for #ident #generics {
            fn to_compound(self) -> simdnbt::owned::NbtCompound {
                let mut nbt = simdnbt::owned::NbtCompound::new();
                #(#field_serializers)*
                nbt
            }
        }
    };

    output.into()
}
