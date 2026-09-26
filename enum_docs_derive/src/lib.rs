use proc_macro::TokenStream;
use quote::quote;
use syn::Attribute;
use syn::Data;
use syn::DeriveInput;
use syn::Expr;
use syn::ExprLit;
use syn::Lit;
use syn::LitStr;
use syn::parse_macro_input;

#[proc_macro_derive(EnumDocs)]
pub fn derive_enum_docs(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;

    let variants = match input.data {
        Data::Enum(data) => data.variants,
        _ => {
            return syn::Error::new_spanned(
                enum_name,
                "EnumDocs can only be derived for enums",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut description_arms = Vec::new();

    for variant in variants {
        let variant_ident = variant.ident;

        if !matches!(variant.fields, syn::Fields::Unit) {
            return syn::Error::new_spanned(
                variant_ident,
                "EnumDocs currently supports only unit variants",
            )
            .to_compile_error()
            .into();
        }

        let description = extract_doc_comment(&variant.attrs);

        description_arms.push(quote! {
            Self::#variant_ident => #description
        });
    }

    let expanded = quote! {
        impl #enum_name {
            pub const fn doc(&self) -> &'static str {
                match self {
                    #(#description_arms),*
                }
            }
        }
    };

    expanded.into()
}

fn extract_doc_comment(attrs: &[Attribute]) -> LitStr {
    let lines: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc")
                && let Ok(meta) = attr.meta.require_name_value()
                && let Expr::Lit(ExprLit {
                    lit: Lit::Str(value),
                    ..
                }) = &meta.value
            {
                Some(value.value().trim_start().to_owned())
            } else {
                None
            }
        })
        .collect();

    LitStr::new(&lines.join(" "), proc_macro2::Span::call_site())
}
