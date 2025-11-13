use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// This will generate a method that extracts accounts from remaining_accounts
/// in the order they're defined in the struct.
#[proc_macro_derive(ExtractAccounts)]
pub fn extract_accounts_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Extract field names from the struct
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("ExtractAccounts only supports structs with named fields"),
        },
        _ => panic!("ExtractAccounts only supports structs"),
    };

    // Generate the field extraction code
    let field_extractions = fields.iter().enumerate().map(|(idx, field)| {
        let field_name = &field.ident;
        quote! {
            #field_name: remaining_accounts
                .get(#idx)
                .cloned()
                .ok_or_else(|| {
                    anchor_lang::prelude::msg!("Missing account at index: {}", #idx);
                    crate::BridgeError::MissingOptionalAccount
                })?
        }
    });

    let expanded = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn extract_from_remaining_accounts(
                remaining_accounts: &[anchor_lang::prelude::AccountInfo<'info>],
            ) -> anchor_lang::prelude::Result<Self> {
                Ok(Self {
                    #(#field_extractions),*
                })
            }
        }
    };

    TokenStream::from(expanded)
}
