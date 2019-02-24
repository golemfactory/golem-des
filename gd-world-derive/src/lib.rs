#![warn(clippy::all)]

extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(DerefDefenceMechanismCommon)]
pub fn deref_defence_mechanism_common_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    impl_deref_defence_mechanism_common(&ast)
}

fn impl_deref_defence_mechanism_common(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl std::ops::Deref for #name {
            type Target = DefenceMechanismCommon;

            fn deref(&self) -> &Self::Target {
                self.as_dm_common()
            }
        }

        impl std::ops::DerefMut for #name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.as_dm_common_mut()
            }
        }
    };

    gen.into()
}

#[proc_macro_derive(DerefProviderCommon)]
pub fn deref_provider_common_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    impl_deref_provider_common(&ast)
}

fn impl_deref_provider_common(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl std::ops::Deref for #name {
            type Target = ProviderCommon;

            fn deref(&self) -> &Self::Target {
                self.as_provider_common()
            }
        }

        impl DerefMut for #name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.as_provider_common_mut()
            }
        }
    };

    gen.into()
}
