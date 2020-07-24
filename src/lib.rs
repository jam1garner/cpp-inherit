use std::ops::Deref;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Fields, ImplItem, Path};

mod method_helpers;
use method_helpers::{filter_overrides, make_extern_c, remove_override_attr};

mod parsers;
use parsers::{InheritImplAttr, NamedField};

mod vtable;
use vtable::generate_vtable_const;

#[proc_macro_attribute]
pub fn inherit_from(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut struct_def = syn::parse_macro_input!(item as syn::ItemStruct);
    let ty = syn::parse_macro_input!(attr as syn::Type);

    let fields = match struct_def.fields {
        Fields::Named(ref mut fields) => &mut fields.named,
        _ => todo!(),
    };

    let base_field: NamedField = syn::parse_quote!(
        _base: #ty
    );

    fields.insert(0, base_field.0);

    let struct_name = &struct_def.ident;

    struct_def.attrs.push(syn::parse_quote! {
        #[repr(C)]
    });

    quote!(
        #struct_def

        impl ::core::ops::Deref for #struct_name {
            type Target = #ty;

            fn deref(&self) -> &Self::Target {
                &self._base
            }
        }

        impl ::core::ops::DerefMut for #struct_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self._base
            }
        }
    )
    .into()
}

fn into_path_segment(ident: &&syn::Ident) -> syn::PathSegment {
    (*ident).clone().into()
}

#[proc_macro_attribute]
pub fn inherit_from_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl_block = syn::parse_macro_input!(item as syn::ItemImpl);
    let InheritImplAttr { class, header, .. } = syn::parse_macro_input!(attr as InheritImplAttr);

    let header = header.value();

    // List of methods with #[overridden] attrbiute
    let mut override_items = impl_block
        .items
        .iter_mut()
        .filter_map(|item| {
            if let ImplItem::Method(ref mut method) = item {
                filter_overrides(method)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Make all override methods `extern "C"`
    override_items.iter_mut().for_each(make_extern_c);

    // Remove fake overridden attributes
    override_items.iter_mut().for_each(remove_override_attr);

    let vtable_info = vtable::get_vtable_info(&header, &class.to_string());

    // List of method override names
    let override_list = override_items
        .into_iter()
        .map(|method| method.sig.ident.clone())
        .collect::<Vec<_>>();

    let type_ident = match *impl_block.self_ty {
        syn::Type::Path(ref path) => path.path.get_ident().unwrap(),
        _ => todo!(), // Error about how class type must be ident
    };

    match vtable_info.get(&class.to_string()) {
        Some(base_type_vtable) => {
            // Generate a vtable before overrides
            let base_vtable: Vec<Option<Path>> = vec![None; base_type_vtable.len()];

            let mut vtable = base_vtable;

            // Apply each override to the base vtable
            for o in override_list {
                match base_type_vtable.binary_search_by_key(&&o.to_string(), |entry| &entry.name) {
                    Ok(index) => {
                        vtable[index] = Some(Path {
                            leading_colon: None,
                            //          $class::$method
                            segments: [&type_ident, &o].iter().map(into_path_segment).collect(),
                        });
                    }
                    Err(..) => todo!(), // add compiler error for overriding a non-existing method
                }
            }

            let mut bindings_to_gen = vec![];

            let vtable = vtable
                .into_iter()
                .enumerate()
                .map(|(i, x)| {
                    x.unwrap_or_else(|| {
                        bindings_to_gen.push(base_type_vtable[i].default.deref());

                        vtable::get_binding_symbol(&base_type_vtable[i].default).into()
                    })
                })
                .collect();

            let self_type = &impl_block.self_ty;

            let vtable_const = generate_vtable_const(vtable, self_type);

            let bindings = bindings_to_gen.into_iter().map(vtable::generate_binding);

            quote!(
                #impl_block

                #vtable_const

                #(
                    #bindings
                )*
            )
            .into()
        }
        None => todo!(), // add compiler error for class not existing in header
    }
}
