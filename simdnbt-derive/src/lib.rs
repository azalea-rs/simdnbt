mod attrs;

use attrs::{parse_field_attrs, parse_unit_attrs};
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Type};

#[proc_macro_derive(Deserialize, attributes(simdnbt))]
pub fn deserialize_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    struct FieldData {
        rust_type: Type,
        field_name: syn::Ident,
        nbt_name: String,
        debug_name: String,
        is_flatten: bool,
    }

    let mut field_datas = Vec::<FieldData>::new();

    // let mut field_deserializers = Vec::<proc_macro2::TokenStream>::new();
    // let mut fields

    match input.data {
        syn::Data::Struct(syn::DataStruct { fields, .. }) => match fields {
            syn::Fields::Named(syn::FieldsNamed { named, .. }) => {
                for field in named {
                    let struct_field_name = field.ident.unwrap();

                    let mut field_attrs = parse_field_attrs(&field.attrs);

                    let nbt_name = field_attrs
                        .rename
                        .take()
                        .unwrap_or_else(|| struct_field_name.to_string());

                    let debug_ident = format!("{ident}::{struct_field_name}");

                    field_datas.push(FieldData {
                        rust_type: field.ty,
                        field_name: struct_field_name,
                        nbt_name: nbt_name.clone(),
                        debug_name: debug_ident,
                        is_flatten: field_attrs.flatten,
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
    let data_lifetime =
        generics
            .lifetimes()
            .next()
            .cloned()
            .unwrap_or_else(|| syn::LifetimeParam {
                attrs: Vec::new(),
                lifetime: syn::Lifetime::new("'_", proc_macro2::Span::call_site()),
                colon_token: None,
                bounds: syn::punctuated::Punctuated::new(),
            });

    let where_clause = &generics.where_clause;

    let struct_attrs = attrs::parse_struct_attrs(&input.attrs);
    let deny_unknown_fields = struct_attrs.deny_unknown_fields;

    // let extra_checks = if struct_attrs.deny_unknown_fields {
    //     quote! {
    //         if !nbt.is_empty() {
    //             return
    // Err(simdnbt::DeserializeError::UnknownField(nbt.keys().next().unwrap().
    // to_string()));         }
    //     }
    // } else {
    //     quote! {}
    // };

    // Option<String>,
    // Option<Vec<String>>,
    // Option<Option<i32>>,
    // simdnbt::owned::NbtCompound,
    let mut partial_type_inner = quote! {};
    for field_data in &field_datas {
        let field_type = &field_data.rust_type;
        partial_type_inner.extend(if field_data.is_flatten {
            quote! { <#field_type as simdnbt::Deserialize<'PARTIAL>>::Partial<'PARTIAL>, }
        } else {
            quote! { Option<#field_type>, }
        });
    }

    // if tag_name == "Name" {
    //     "ItemDisplay::name"
    // } else if tag_name == "Lore" {
    //     "ItemDisplay::lore"
    // } else if tag_name == "color" {
    //     "ItemDisplay::color"
    // } else
    let mut name_to_field_if_statements = quote! {};
    for field_data in &field_datas {
        let nbt_name = &field_data.nbt_name;
        let debug_name = &field_data.debug_name;
        name_to_field_if_statements.extend(quote! {
            if tag_name == #nbt_name {
                #debug_name
            } else
        });
    }

    // name: partial
    //     .0
    //     .ok_or(simdnbt::DeserializeError::MissingField("ItemDisplay::name"))?,
    // lore: partial
    //     .1
    //     .ok_or(simdnbt::DeserializeError::MissingField("ItemDisplay::lore"))?,
    // color: partial.2.ok_or(simdnbt::DeserializeError::MissingField(
    //     "ItemDisplay::color",
    // ))?,
    // _extra: partial.3,
    let mut construct_fields = quote! {};
    for (i, field_data) in field_datas.iter().enumerate() {
        let field_name = &field_data.field_name;
        let debug_name = &field_data.debug_name;
        let rust_type = &field_data.rust_type;
        let index = proc_macro2::Literal::usize_unsuffixed(i);
        construct_fields.extend(if field_data.is_flatten {
            quote! {
                #field_name: simdnbt::Deserialize::from_partial(partial.#index)?,
            }
        } else {
            quote! {
                #field_name: <#rust_type>::try_flatten_with_option(partial.#index).ok_or(simdnbt::DeserializeError::MissingField(#debug_name))?,
            }
        });
    }

    // if <String>::type_matches(tag_type) && tag_name == "Name" {
    //     partial.0 = Some(simdnbt::Deserialize::read_value_direct(data)?);
    // } else if <Vec<String>>::type_matches(tag_type) && tag_name == "Lore" {
    //     partial.1 = Some(simdnbt::Deserialize::read_value_direct(data)?);
    // } else if <i32>::type_matches(tag_type) && tag_name == "color" {
    //     partial.2 = Some(simdnbt::Deserialize::read_value_direct(data)?);
    // } else if <simdnbt::owned::NbtCompound>::update_partial(
    //     &mut partial.3,
    //     tag_name,
    //     tag_type,
    //     data,
    // )? {
    // } else
    let mut update_partial_inner = quote! {};
    for (i, field_data) in field_datas.iter().enumerate() {
        let field_type = &field_data.rust_type;
        let index = proc_macro2::Literal::usize_unsuffixed(i);
        let nbt_name = &field_data.nbt_name;
        update_partial_inner.extend(if field_data.is_flatten {
            quote! {
                if <#field_type>::update_partial(&mut partial.#index, tag_name, tag_type, data)? {
                } else
            }
        } else {
            quote! {
                if <#field_type>::type_matches(tag_type) && tag_name == #nbt_name {
                    partial.#index = Some(simdnbt::Deserialize::read_value_direct_with_explicit_type(data, tag_type)?);
                } else
            }
        });
    }

    let output = quote! {
        impl #generics simdnbt::Deserialize<#data_lifetime> for #ident #generics #where_clause {
            const NBT_TYPE_ID: u8 = simdnbt::common::COMPOUND_ID;
            type Partial<'PARTIAL> = (#partial_type_inner);

            #[inline]
            fn read_value_direct(data: &mut simdnbt::reader::Reader<#data_lifetime>) -> Result<Self, simdnbt::DeserializeError> {
                let mut partial = Self::Partial::default();

                loop {
                    let tag_type = data.read_u8()?;
                    if tag_type == simdnbt::common::END_ID {
                        break;
                    }
                    let tag_name = simdnbt::common::read_string(data)?;

                    let matched = Self::update_partial(&mut partial, tag_name, tag_type, data)?;

                    if !matched {
                        let field_name = #name_to_field_if_statements {
                            if #deny_unknown_fields {
                                return Err(simdnbt::DeserializeError::UnknownField(
                                    tag_name.to_str().into(),
                                ));
                            }

                            // skip the field
                            simdnbt::validate::internal_read_tag(data, tag_type)?;

                            continue;
                        };

                        return Err(simdnbt::DeserializeError::MismatchedFieldType(field_name));
                    }
                }

                Self::from_partial(partial)
            }

            #[inline]
            fn update_partial(
                partial: &mut Self::Partial<'_>,
                tag_name: &simdnbt::Mutf8Str,
                tag_type: u8,
                data: &mut simdnbt::reader::Reader<#data_lifetime>,
            ) -> Result<bool, simdnbt::DeserializeError> {
                #update_partial_inner {
                    return Ok(false);
                }

                Ok(true)
            }

            #[inline]
            fn from_partial(partial: Self::Partial<'_>) -> Result<Self, simdnbt::DeserializeError> {
                Ok(Self {
                    #construct_fields
                })
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
    let where_clause = &generics.where_clause;

    let output = quote! {
        impl #generics simdnbt::Serialize for #ident #generics #where_clause {
            fn to_compound(self) -> simdnbt::owned::NbtCompound {
                let mut nbt = simdnbt::owned::NbtCompound::new();
                #(#field_serializers)*
                nbt
            }
        }
    };

    output.into()
}

#[proc_macro_derive(FromNbtTag, attributes(simdnbt))]
pub fn from_nbt_tag_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let mut matchers = Vec::<proc_macro2::TokenStream>::new();

    match input.data {
        syn::Data::Struct(_) => panic!("Use #[derive(Deserialize)] instead"),
        syn::Data::Enum(syn::DataEnum { variants, .. }) => {
            for variant in variants {
                match variant.fields {
                    syn::Fields::Named(_) => todo!(),
                    syn::Fields::Unnamed(_) => todo!(),
                    syn::Fields::Unit => {
                        let enum_variant_name = variant.ident;

                        let mut unit_attrs = parse_unit_attrs(&variant.attrs);

                        let variant_name = unit_attrs
                            .rename
                            .take()
                            .unwrap_or_else(|| enum_variant_name.to_string());

                        matchers.push(quote! {
                            #variant_name => Some(Self::#enum_variant_name),
                        });
                    }
                }
            }
        }
        syn::Data::Union(_) => todo!(),
    }

    let generics = input.generics;
    let where_clause = &generics.where_clause;

    let output = quote! {
        impl #generics simdnbt::FromNbtTag for #ident #generics #where_clause {
            fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
                match tag.string()?.to_str().as_ref() {
                    #(#matchers)*
                    _ => None,
                }
            }
        }
    };

    output.into()
}

#[proc_macro_derive(ToNbtTag, attributes(simdnbt))]
pub fn to_nbt_tag_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let mut field_matchers = Vec::<proc_macro2::TokenStream>::new();

    match input.data {
        syn::Data::Struct(_) => panic!("Use #[derive(Serialize)] instead"),
        syn::Data::Enum(syn::DataEnum { variants, .. }) => {
            for variant in variants {
                match variant.fields {
                    syn::Fields::Named(_) => todo!(),
                    syn::Fields::Unnamed(_) => todo!(),
                    syn::Fields::Unit => {
                        let enum_variant_name = variant.ident;

                        let mut unit_attrs = parse_unit_attrs(&variant.attrs);

                        let variant_name = unit_attrs
                            .rename
                            .take()
                            .unwrap_or_else(|| enum_variant_name.to_string());

                        field_matchers.push(quote! {
                            Self::#enum_variant_name => simdnbt::owned::NbtTag::String(#variant_name.into()),
                        });
                    }
                }
            }
        }
        syn::Data::Union(_) => todo!(),
    }

    let generics = input.generics;
    let where_clause = &generics.where_clause;

    let output = quote! {
        impl #generics simdnbt::ToNbtTag for #ident #generics #where_clause {
            fn to_nbt_tag(self) -> simdnbt::owned::NbtTag {
                match self {
                    #(#field_matchers)*
                }
            }
        }
    };

    output.into()
}
