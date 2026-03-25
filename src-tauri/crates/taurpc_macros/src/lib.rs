use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Attribute, FnArg, ItemTrait, Pat, TraitItem, TraitItemFn,
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

#[proc_macro_attribute]
pub fn taurpc_api(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_trait = parse_macro_input!(item as ItemTrait);
    let trait_ident = item_trait.ident.clone();
    let procedures_args = proc_macro2::TokenStream::from(attr);

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
                self::resolvers::#name::#name(self, #(#arg_idents),*).await
            }
        });
    }

    let procedures_attr = if procedures_args.is_empty() {
        quote! {
            #[taurpc::procedures]
        }
    } else {
        quote! {
            #[taurpc::procedures(#procedures_args)]
        }
    };

    TokenStream::from(quote! {
        #procedures_attr
        #item_trait

        pub mod resolvers {
            #(#resolver_mod_items)*
        }

        #[taurpc::resolvers]
        impl #trait_ident for ApiImpl {
            #(#impl_methods)*
        }
    })
}