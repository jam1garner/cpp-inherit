use syn::{Type, Path};
use quote::{quote, ToTokens};

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
