use syn::Attribute;

use crate::parse::ItemClass;

pub trait AttributeExtractor {
    type Output;

    /// Extracts an attribute out of a class.
    fn extract(class: &mut ItemClass) -> Option<Self::Output> {
        // see if there's a `derive` attribute
        let gen_base_idx = class
            .attrs
            .iter()
            .position(|attr| attr.path().is_ident(Self::attr()))?;
        let gen_base_attr = class.attrs.remove(gen_base_idx);

        Some(
            Self::parse_attr(gen_base_attr)
                .unwrap_or_else(|e| panic!("Parse `{}` error: {e}", Self::attr())),
        )
    }

    /// Returns the attribute name.
    fn attr() -> &'static str;
    /// Parses the attribute itself.
    fn parse_attr(attr: Attribute) -> syn::Result<Self::Output>;
}
