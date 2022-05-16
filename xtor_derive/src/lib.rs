extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, DeriveInput, Error, Meta, NestedMeta};

/// Xtor Message Derive
///
/// # Examples
/// ```ignore
/// #[message(result = "i32")]
/// struct TestMessage(i32);
/// ```
#[proc_macro_attribute]
pub fn message(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let mut result_type = quote! { () };

    for arg in args {
        if let NestedMeta::Meta(Meta::NameValue(nv)) = arg {
            if nv.path.is_ident("result") {
                if let syn::Lit::Str(lit) = nv.lit {
                    if let Ok(ty) = syn::parse_str::<syn::Type>(&lit.value()) {
                        result_type = quote! { #ty };
                    } else {
                        return Error::new_spanned(
                            &lit,
                            format!("Expect type found {:?}", lit.value()),
                        )
                        .to_compile_error()
                        .into();
                    }
                }
            }
        }
    }

    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let expanded = quote! {
        #input
        impl xtor::actor::message::Message for #ident {
            type Result = #result_type;
        }
    };
    expanded.into()
}

/// Xtor main derive
/// # Examples
/// ```ignore
/// #[xtor::main]
/// async fn main() {
///    println!("Hello, world!");
/// }
/// ```
///
#[proc_macro_attribute]
pub fn main(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = syn::parse_macro_input!(input as syn::ItemFn);

    if &*input.sig.ident.to_string() != "main" {
        return TokenStream::from(quote_spanned! { input.sig.ident.span() =>
            compile_error!("only the main function can be tagged with #[xtor::main]"),
        });
    }

    if input.sig.asyncness.is_none() {
        return TokenStream::from(quote_spanned! { input.span() =>
            compile_error!("only allow async functions to be tagged with #[xtor::main]"),
        });
    }

    input.sig.ident = Ident::new("__main", Span::call_site());
    let ret = &input.sig.output;

    let expanded = quote! {
        #input

        fn main() #ret {
            xtor::block_on(async{let res = __main().await; xtor::await_exit().await; res})
        }
    };

    expanded.into()
}

/// Xtor test derive
/// # Examples
/// ```ignore
/// #[xtor::test]
/// async fn test_xxx() {
///    println!("Hello, world!");
///    assert!(1 == 1);
/// }
/// ```
///
#[proc_macro_attribute]
pub fn test(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = syn::parse_macro_input!(input as syn::ItemFn);

    let ident = &input.sig.ident.clone();

    if input.sig.asyncness.is_none() {
        return TokenStream::from(quote_spanned! { input.span() =>
            compile_error!("only allow async functions to be tagged with #[xtor::test]"),
        });
    }

    input.sig.ident = Ident::new("__test", Span::call_site());
    let ret = &input.sig.output;

    let expanded = quote! {
        #[test]
        fn #ident() #ret {
            #input
            xtor::block_on(async{let res = __test().await; xtor::await_exit().await; res})
        }
    };

    expanded.into()
}
