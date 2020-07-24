use syn::{Attribute, ImplItemMethod};

fn is_override_attr(attr: &Attribute) -> bool {
    attr.path
        .get_ident()
        .map(|ident| ident.to_string() == "overridden")
        .unwrap_or(false)
}

pub fn filter_overrides(method: &mut ImplItemMethod) -> Option<&mut ImplItemMethod> {
    if method.attrs.iter().any(is_override_attr) {
        Some(method)
    } else {
        None
    }
}

pub fn remove_override_attr(method: &mut &mut ImplItemMethod) {
    method.attrs.retain(|attr| !is_override_attr(attr));
}

pub fn make_extern_c(method: &mut &mut ImplItemMethod) {
    method.sig.abi.replace(syn::parse_quote!(extern "C"));
}
