// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use crate::{peripherals::aes, Capsule, Component};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "aes")]
pub struct AesCapsule<A: aes::Aes + 'static> {
    peripheral: Rc<A>,
    number_of_blocks: usize,
}

impl<A: aes::Aes + 'static> AesCapsule<A> {
    #[inline]
    pub fn get(peripheral: Rc<A>, number_of_blocks: usize) -> Rc<Self> {
        Rc::new(Self::new(peripheral, number_of_blocks))
    }
}

impl<A: aes::Aes> Component for AesCapsule<A> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;

        Ok(quote::quote!(
            capsules_extra::symmetric_encryption::aes::AesDriver<
                'static,
                capsules_aes_gcm::aes_gcm::Aes128Gcm<
                    'static,
                    capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, #peripheral_ty>,
                >
            >
        ))
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        let number_of_blocks = self.number_of_blocks;
        let peripheral_ty = self.peripheral.ty().unwrap();
        let peripheral_identifier: proc_macro2::TokenStream =
            self.peripheral.ident().unwrap().parse().unwrap();

        Some(quote::quote! {
            const CRYPT_SIZE: usize = #number_of_blocks * kernel::hil::symmetric_encryption::AES128_BLOCK_SIZE;

            let ccm_mux = kernel::static_init!(
                capsules_core::virtualizers::virtual_aes_ccm::MuxAES128CCM<'static, #peripheral_ty>,
                capsules_core::virtualizers::virtual_aes_ccm::MuxAES128CCM::new(&#peripheral_identifier),
            );
            kernel::deferred_call::DeferredCallClient::register(ccm_mux);
            kernel::hil::symmetric_encryption::AES128::set_client(&#peripheral_identifier, ccm_mux);

            let ccm_client = components::aes::AesVirtualComponent::new(ccm_mux).finalize(
                components::aes_virtual_component_static!(#peripheral_ty)
            );

            let crypt_buf2 = kernel::static_init!(
                [u8; CRYPT_SIZE],
                [0x00; CRYPT_SIZE],
            );

            let gcm_client = kernel::static_init!(
                capsules_aes_gcm::aes_gcm::Aes128Gcm<
                    'static,
                    capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, #peripheral_ty>
                >,
                capsules_aes_gcm::aes_gcm::Aes128Gcm::new(ccm_client, crypt_buf2),
            );
            kernel::hil::symmetric_encryption::AES128::set_client(gcm_client, ccm_client);
        })
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let driver_num = self.driver_num();
        let peripheral_ty = self.peripheral.ty()?;

        Ok(quote::quote!(
            components::aes::AesDriverComponent::new(
                board_kernel,
                #driver_num,
                gcm_client
            )
            .finalize(components::aes_driver_component_static!(
                capsules_aes_gcm::aes_gcm::Aes128Gcm<
                    'static,
                    capsules_core::virtualizers::virtual_aes_ccm::VirtualAES128CCM<'static, #peripheral_ty>,
                >
            ));
        ))
    }
}

impl<A: aes::Aes> Capsule for AesCapsule<A> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::symmetric_encryption::aes::DRIVER_NUM)
    }
}
