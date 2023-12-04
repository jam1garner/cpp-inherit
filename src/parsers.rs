use syn::{Field, Ident, LitStr, Token};

pub struct InheritImplAttr {
    pub class: Ident,
    _comma: Token![,],
    pub header: LitStr,
}

impl syn::parse::Parse for InheritImplAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            class: input.parse()?,
            _comma: input.parse()?,
            header: input.parse()?,
        })
    }
}

pub struct NamedField(pub Field);

impl syn::parse::Parse for NamedField {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self(Field::parse_named(input)?))
    }
}

impl From<NamedField> for Field {
    fn from(val: NamedField) -> Self {
        val.0
    }
}
