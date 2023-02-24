use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

/// Declare a `VrfState` wrapper struct for [`VrfAccountData`].
/// We willuse this struct to interact with the `vrf_sdk`
/// through [`request_randomness`] function.
///
/// User should use this immediately after anchor `declare_id!()` macro.
///
/// Example
/// ```ignore
/// use anchor_lang::prelude::*;
///
/// declare_id!("6trpiXViFkrXFR1F1nMDGMyigUo89c53La2Bpc4mMwyG");
/// vrf_sdk::declare_vrf_state!(VrfState);
/// ```
///
/// We do this because anchor_lang::AccountLoader<'info, T> required
/// that we implements anchor_lang::Owner, which is the program_id
/// but we just a library and implement that is not trivial.
/// So, as a work around we make a wrapper struct in the program itself
/// via macro.
#[proc_macro]
pub fn declare_vrf_state(item: TokenStream) -> TokenStream {
    let struct_name = syn::parse_macro_input!(item as Ident);
    let struct_name_str = struct_name.to_string();

    quote! {
        /// Wrapper struct for `vrf_sdk::VrfAccountData`
        ///
        /// Each randomness request should create a new instance of this
        /// struct on-chain, and call (request_randomness)[`vrf_sdk::request_randomness`]
        #[derive(Clone, Copy)]
        #[repr(packed)]
        pub struct #struct_name {
            vrf: ::vrf_sdk::vrf::VrfAccountData,
        }

        #[automatically_derived]
        unsafe impl ::vrf_sdk::__private::Pod for #struct_name {}

        #[automatically_derived]
        unsafe impl ::vrf_sdk::__private::Zeroable for #struct_name {}

        #[automatically_derived]
        impl ::vrf_sdk::__private::ZeroCopy for #struct_name {}

        #[automatically_derived]
        impl ::vrf_sdk::__private::Owner for #struct_name {
            fn owner() -> ::vrf_sdk::__private::Pubkey {
                crate::ID
            }
        }

        #[automatically_derived]
        impl ::vrf_sdk::__private::Discriminator for #struct_name {
            const DISCRIMINATOR: [u8; 8] = ::vrf_sdk::vrf::VrfAccountData::DISCRIMINATOR;
        }

        #[automatically_derived]
        impl std::ops::Deref for #struct_name {
            type Target = vrf_sdk::vrf::VrfAccountData;

            fn deref(&self) -> &Self::Target {
                &self.vrf
            }
        }

        #[automatically_derived]
        impl std::ops::DerefMut for #struct_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.vrf
            }
        }

		#[automatically_derived]
		impl ::vrf_sdk::__private::AccountDeserialize for #struct_name {
			fn try_deserialize(buf: &mut &[u8]) -> ::vrf_sdk::__private::Result<Self> {
				if buf.len() < <Self as ::vrf_sdk::__private::Discriminator>::DISCRIMINATOR.len() {
					return Err(::vrf_sdk::__private::error::ErrorCode::AccountDiscriminatorNotFound.into());
				}
				let given_disc = &buf[..8];
				if &<Self as ::vrf_sdk::__private::Discriminator>::DISCRIMINATOR != given_disc {
					return Err(::vrf_sdk::__private::error!(::vrf_sdk::__private::error::ErrorCode::AccountDiscriminatorMismatch).with_account_name(#struct_name_str));
				}
				Self::try_deserialize_unchecked(buf)
			}

			fn try_deserialize_unchecked(buf: &mut &[u8]) -> ::vrf_sdk::__private::Result<Self> {
				let data: &[u8] = &buf[8..];
				// Re-interpret raw bytes into the POD data structure.
				let account = ::vrf_sdk::__private::from_bytes(data);
				// Copy out the bytes into a new, owned data structure.
				Ok(*account)
			}
		}
    }
    .into()
}
