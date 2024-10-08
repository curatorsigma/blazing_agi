//! This crate is a companion to `blazing_agi`. Please see its documentation for more information.
//!
//! We provide proc_macros that enable a neat API in the main crate.
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Expr, ExprTuple, Ident, ItemFn};

/// Given an async fn, create an AGIHandler from it.
///
/// To use this, make sure that the fn has *exactly* this signature (you may use the types, or name
/// them with FQTNs)
/// ```ignore
/// async fn foo(connection: &mut Connection, request: &AGIRequest) -> Result<(), AGIError>
/// ```
/// If you change the signature (even the variable names), your IDE will give you weird auto-completes
/// and type-checking or compilation may fail.
/// If you do not use one of the arguments, you may change their name to `_`.
///
/// Note: What we really want is a transformation: `async fn(&mut Connection, &AGIRequest) -> AGIHandler`.
/// But naming the types (specifically: lifetimes) there is very hard until RPIT captures lifetimes.
/// I decided for this somewhat more hacky solution: simply copy-pasting the function body
/// directly into a new impl block with this macro.
#[proc_macro_attribute]
pub fn create_handler(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);

    let fn_name = input.sig.ident;
    let fn_block = input.block;
    let struct_name = Ident::new(format!("Blazing_AGI_Handler_{fn_name}").as_str(), Span::call_site());

    let tokens = quote! {
        #[derive(Debug,Clone)]
        struct #struct_name {}
        #[::async_trait::async_trait]
        impl ::blazing_agi::handler::AGIHandler for #struct_name {
            async fn handle(&self, connection: &mut ::blazing_agi::connection::Connection, request: &::blazing_agi::AGIRequest) -> Result<(), ::blazing_agi::AGIError> {
                #fn_block
            }
        }
        #[allow(non_upper_case_globals)]
        const #fn_name: #struct_name = #struct_name {};
    };
    tokens.into()
}

/// Chain two handlers. The second will only run if the first returned successfully.
///
/// The input to this macro is a tuple containing two expressions evaluating to an AGIHandler.
#[proc_macro]
pub fn and_then(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ExprTuple);
    let first = &input.elems[0];
    let second = &input.elems[1];
    let tokens = quote! {
        AndThenHandler::new(Box::new(#first), Box::new(#second))
    };
    tokens.into()
}

/// Create an `AndThenLayerBefore` from another
/// handler.
///
/// The input is a Handler.
#[proc_macro]
pub fn layer_before(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Expr);
    quote! {
        ::blazing_agi::layer::AndThenLayerBefore::new(#input)
    }.into()
}
