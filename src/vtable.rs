use syn::{Type, Path, Ident};
use quote::{quote, ToTokens, format_ident};

pub fn generate_vtable_const(methods: Vec<Path>, ty: &Type) -> impl ToTokens {
    let method_count = methods.len();
    quote!(
        impl #ty {
            // One constant to do a static borrow to ensure it's effectively a static
            const _VTABLE_BORROW_FDKSLASDASD: &'static [*const (); #method_count] = &[
                #(
                    #methods as *const (),
                )*
            ];
            
            // One constant to convert to a pointer to reduce casting
            //
            // TODO: is it possible to get the bindgen vtable type? if so then no casting would be
            // needed...
            const VTABLE_: *const [*const (); #method_count] = #ty::_VTABLE_BORROW_FDKSLASDASD as *const _;
        }
    )
}

/// Returns a list of the names of virtual methods, in order of their layout in memory
pub fn get_vtable_order(header: &str, class: &str) -> Vec<String> {
    todo!()
}

pub fn get_base_method(class: &Ident, method: &str) -> Path {
    format_ident!("{}_{}", class, method).into()
}
