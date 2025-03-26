// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::{peripherals::asymmetric_crypto, Capsule, Component, Ident};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "asymmetric_crypto")]
pub struct AsymmetricCryptoCapsule<P: asymmetric_crypto::AsymmetricCrypto + 'static> {
    driver_num_var: String,
    hash_len: String,
    sig_len: String,
    hil: String,
    peripheral: Rc<P>,
}

impl<P: asymmetric_crypto::AsymmetricCrypto + 'static> AsymmetricCryptoCapsule<P> {
    #[inline]
    pub fn get(
        driver_num_var: String,
        hash_len: String,
        sig_len: String,
        hil: String,
        peripheral: Rc<P>,
    ) -> Rc<Self> {
        Rc::new(Self::new(
            driver_num_var,
            hash_len,
            sig_len,
            hil,
            peripheral,
        ))
    }
}

impl<P: asymmetric_crypto::AsymmetricCrypto> Component for AsymmetricCryptoCapsule<P> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn crate::Component>>> {
        Some(vec![self.peripheral.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        let hash_len: proc_macro2::TokenStream = self.hash_len.parse().unwrap();
        let sig_len: proc_macro2::TokenStream = self.sig_len.parse().unwrap();
        Ok(quote::quote!(
            capsules_extra::public_key_crypto::asymmetric_crypto::AsymmetricCrypto<'static, { #hash_len }, { #sig_len }, #peripheral_ty>
        ))
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        let hash_len: proc_macro2::TokenStream = self.hash_len.parse().unwrap();
        let sig_len: proc_macro2::TokenStream = self.sig_len.parse().unwrap();
        Some(quote::quote!(
            let __hash_buf: &'static mut [u8; #hash_len] =
                kernel::static_init!([u8; #hash_len], [0u8; #hash_len],);
            let __sig_buf : &'static mut [u8; #sig_len] =
                kernel::static_init!([u8; #sig_len], [0u8; #sig_len],);
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ident: proc_macro2::TokenStream = self.peripheral.ident()?.parse().unwrap();
        let ty = self.ty()?;
        let driver_num = self.driver_num();

        Ok(quote::quote!(kernel::static_init!(
            #ty,
            capsules_extra::public_key_crypto::asymmetric_crypto::AsymmetricCrypto::new(
                #peripheral_ident,
                __hash_buf,
                __sig_buf,
                board_kernel.create_grant(#driver_num, &memory_allocation_cap)
            ),
        )))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        let hil: proc_macro2::TokenStream = self.hil.parse().unwrap();
        let peripheral_ident: proc_macro2::TokenStream =
            self.peripheral.ident().unwrap().parse().unwrap();

        Some(quote::quote!(
            kernel::hil::public_key_crypto::ecc::#hil::set_verify_client(
                #peripheral_ident,
                #ident,
            );

        ))
    }
}

impl<P: asymmetric_crypto::AsymmetricCrypto> Capsule for AsymmetricCryptoCapsule<P> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        let driver_num_var: proc_macro2::TokenStream = self.driver_num_var.parse().unwrap();
        quote::quote!(capsules_extra::public_key_crypto::asymmetric_crypto::#driver_num_var)
    }
}
