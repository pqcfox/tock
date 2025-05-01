// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::ffi::cryptolib::mux::CryptolibMux;
use crate::peripherals::{Peripheral, NO_PARAM};
use parse::{Component, Error, Ident};
use std::rc::Rc;

macro_rules! cryptolib_asymmetric {
    {$driver_ty:ident, $name:expr} => {
        /// ECDSA implementation using cryptolib
        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        pub struct $driver_ty<T: parse::Timer + 'static> {
            mux: Rc<CryptolibMux<T>>,
        }

        impl<T: parse::Timer + 'static> $driver_ty<T> {
            pub fn new(mux: Rc<CryptolibMux<T>>) -> Self {
                Self { mux }
            }

            pub fn mux(&self) -> Rc<CryptolibMux<T>> {
                self.mux.clone()
            }
        }

        impl<T: parse::Timer + 'static> parse::Ident for $driver_ty<T> {
            fn ident(&self) -> Result<String, parse::Error> {
                Ok(String::from($name))
            }
        }

        impl<T: parse::Timer + 'static> parse::Component for $driver_ty<T> {
            fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
                Some(vec![self.mux.clone()])
            }

            fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
                let timer_ty = self.mux.timeout_mux().mux_alarm().timer().ty()?;
                Ok(quote::quote!(lowrisc::ffi::cryptolib::ecc::ecdsa::$driver_ty<'static, #timer_ty>))
            }

            fn init_expr(&self) -> Result<proc_macro2::TokenStream, Error> {
                let mux_ident: proc_macro2::TokenStream = self.mux.ident()?.parse().unwrap();
                let ty = self.ty()?;

                Ok(quote::quote!(
                    kernel::static_init!(
                        #ty,
                        lowrisc::ffi::cryptolib::ecc::ecdsa::$driver_ty::new(
                            #mux_ident,
                            lowrisc::ffi::cryptolib::timeouts::ECDSA_P256_VERIFY_TIMEOUT.into()
                        ),
                    )
                ))
            }

            fn after_init(&self) -> Option<proc_macro2::TokenStream> {
                let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
                Some(quote::quote!(
                    #ident.set_self_ref();
                ))
            }

            fn trace_dependencies(&self, peripherals: &mut dyn parse::component::ConfigPeripherals) {
                peripherals.require_interrupts(
                    Peripheral::Keymgr as usize,
                    NO_PARAM
                );
                peripherals.require_interrupts(
                    Peripheral::Otbn as usize,
                    NO_PARAM
                )
            }
        }

        impl<T: parse::Timer + 'static> std::fmt::Display for $driver_ty<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $name)
            }
        }

        impl<T: parse::Timer + 'static> parse::peripherals::asymmetric_crypto::AsymmetricCrypto for $driver_ty<T> {}
    }
}

cryptolib_asymmetric! {OtCryptoEcdsaP256, "otcrypto_p256"}
cryptolib_asymmetric! {OtCryptoEcdsaP384, "otcrypto_p384"}
