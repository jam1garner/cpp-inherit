use proc_macro::TokenStream;
use syn::{Fields, ImplItem, Path};
use quote::{quote, format_ident};

mod method_helpers;
use method_helpers::{filter_overrides, remove_override_attr, make_extern_c};

mod parsers;
use parsers::{NamedField, InheritImplAttr};

mod vtable;
use vtable::generate_vtable_const;

#[proc_macro_attribute]
pub fn inherit_from(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut struct_def = syn::parse_macro_input!(item as syn::ItemStruct);
    let ty = syn::parse_macro_input!(attr as syn::Type);

    let fields = match struct_def.fields {
        Fields::Named(ref mut fields) => {
            &mut fields.named
        }
        _ => todo!()
    };

    let base_field: NamedField = syn::parse_quote!(
        _base: #ty
    );

    fields.insert(0, base_field.0);

    let struct_name = &struct_def.ident;

    struct_def.attrs.push(syn::parse_quote!{
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
    ).into()
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
    let mut override_items =
        impl_block
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
    override_items
        .iter_mut()
        .for_each(make_extern_c);

    // Remove fake overridden attributes
    override_items
        .iter_mut()
        .for_each(remove_override_attr);

    let vtable_order = vtable::get_vtable_order(&header, &class.to_string());

    // Generate a vtable before overrides
    let base_vtable =
        vtable_order
            .iter()
            .map(|method| vtable::get_base_method(&class, &method))
            .collect::<Vec<_>>();

    // List of method override names
    let override_list = 
        override_items
            .into_iter()
            .map(|method| method.sig.ident.clone())
            .collect::<Vec<_>>();

    let mut vtable = base_vtable;

    let type_ident = match *impl_block.self_ty {
        syn::Type::Path(ref path) => path.path.get_ident().unwrap(),
        _ => todo!() // Error about how class type must be ident
    };

    // Apply each override to the base vtable
    for o in override_list {
        match vtable_order.binary_search(&o.to_string()) {
            Ok(index) => {
                vtable[index] = Path {
                    leading_colon: None,
                    // $class::$method
                    segments: [&type_ident, &o].iter().map(into_path_segment).collect()
                };
            }
            Err(..) => todo!(), // add compiler error for overriding a non-existant virtual method
        }
    }

    let self_type = &impl_block.self_ty;

    let vtable_const = generate_vtable_const(vtable, self_type);

    quote!(
        #impl_block

        #vtable_const
    ).into()
}

