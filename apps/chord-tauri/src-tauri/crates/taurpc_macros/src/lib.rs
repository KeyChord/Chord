use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Attribute, FnArg, ItemTrait, LitStr, Pat, TraitItem, TraitItemFn, parse_macro_input,
    parse_quote,
};

fn has_taurpc_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("taurpc"))
}

fn has_taurpc_skip(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("taurpc"))
        .any(|attr| {
            let mut skip = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    skip = true;
                }
                Ok(())
            });
            skip
        })
}

fn extract_arg_idents(method: &TraitItemFn) -> syn::Result<Vec<syn::Ident>> {
    let mut idents = Vec::new();

    for input in &method.sig.inputs {
        match input {
            FnArg::Typed(pat_ty) => match &*pat_ty.pat {
                Pat::Ident(pat_ident) => idents.push(pat_ident.ident.clone()),
                _ => {
                    return Err(syn::Error::new_spanned(
                        &pat_ty.pat,
                        "taurpc_api methods must use simple identifier arguments like `bundle_id: String`",
                    ));
                }
            },
            FnArg::Receiver(receiver) => {
                return Err(syn::Error::new_spanned(
                    receiver,
                    "taurpc_api trait methods must not declare `self`",
                ));
            }
        }
    }

    Ok(idents)
}

fn parse_module_ident(lit: &LitStr) -> syn::Result<syn::Ident> {
    syn::parse_str::<syn::Ident>(&lit.value()).map_err(|_| {
        syn::Error::new_spanned(
            lit,
            "`mod` must be a valid Rust identifier string, for example \"resolvers\"",
        )
    })
}

#[proc_macro_attribute]
pub fn taurpc_api(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut export_to: Option<LitStr> = None;
    let mut resolver_mod_name: Option<LitStr> = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("export_to") {
            let value: LitStr = meta.value()?.parse()?;
            if export_to.replace(value).is_some() {
                return Err(meta.error("duplicate `export_to` argument"));
            }
            return Ok(());
        }

        if meta.path.is_ident("mod") {
            let value: LitStr = meta.value()?.parse()?;
            if resolver_mod_name.replace(value).is_some() {
                return Err(meta.error("duplicate `mod` argument"));
            }
            return Ok(());
        }

        Err(meta.error("unsupported argument; expected `export_to = \"...\"` or `mod = \"...\"`"))
    });

    parse_macro_input!(attr with parser);

    let resolver_mod_ident = match resolver_mod_name {
        Some(lit) => match parse_module_ident(&lit) {
            Ok(ident) => ident,
            Err(error) => return error.to_compile_error().into(),
        },
        None => syn::Ident::new("resolvers", Span::call_site()),
    };

    let mut item_trait = parse_macro_input!(item as ItemTrait);
    let trait_ident = item_trait.ident.clone();

    let mut resolver_mod_items = Vec::new();
    let mut impl_methods = Vec::new();

    for item in &mut item_trait.items {
        let TraitItem::Fn(method) = item else {
            continue;
        };

        let is_skipped = has_taurpc_skip(&method.attrs);

        if !has_taurpc_attr(&method.attrs) {
            let alias = method.sig.ident.to_string().to_case(Case::Camel);
            let alias_attr: syn::Attribute = parse_quote! {
                #[taurpc(alias = #alias)]
            };
            method.attrs.push(alias_attr);
        }

        if is_skipped {
            continue;
        }

        let name = method.sig.ident.clone();

        let arg_idents = match extract_arg_idents(method) {
            Ok(arg_idents) => arg_idents,
            Err(error) => return error.to_compile_error().into(),
        };

        let mut impl_sig = method.sig.clone();
        impl_sig.inputs.insert(0, parse_quote!(self));

        resolver_mod_items.push(quote! {
            pub mod #name;
        });

        impl_methods.push(quote! {
            #impl_sig {
                self::#resolver_mod_ident::#name::#name(self, #(#arg_idents),*).await
            }
        });
    }

    let procedures_attr = match export_to {
        Some(path) => {
            quote! {
                #[taurpc::procedures(export_to = #path)]
            }
        }
        None => {
            quote! {
                #[taurpc::procedures]
            }
        }
    };

    TokenStream::from(quote! {
        #procedures_attr
        #item_trait

        pub mod #resolver_mod_ident {
            #(#resolver_mod_items)*
        }

        #[taurpc::resolvers]
        impl #trait_ident for ApiImpl {
            #(#impl_methods)*
        }
    })
}
