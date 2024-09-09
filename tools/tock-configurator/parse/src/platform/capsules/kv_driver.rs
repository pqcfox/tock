// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use crate::peripherals::flash::{self, Page};
use crate::{Capsule, Component};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "kv_driver")]
pub struct KvDriver<F: flash::Flash + 'static> {
    peripheral: Rc<F>,
}

impl<F: flash::Flash + 'static> KvDriver<F> {
    #[inline]
    pub fn get(peripheral: Rc<F>) -> Rc<Self> {
        Rc::new(Self::new(peripheral))
    }
}
impl<F: flash::Flash> Component for KvDriver<F> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        let page_size = F::Page::size();

        Ok(quote::quote!(
            capsules_extra::kv_driver::KVStoreDriver<
                'static,
                capsules_extra::virtual_kv::VirtualKVPermissions<
                    'static,
                    capsules_extra::kv_store_permissions::KVStorePermissions<
                        'static,
                        capsules_extra::tickv_kv_store::TicKVKVStore<
                            'static,
                            capsules_extra::tickv::TicKVSystem<
                                'static,
                                capsules_core::virtualizers::virtual_flash::FlashUser<
                                    'static,
                                    #peripheral_ty,
                                >,
                                capsules_extra::sip_hash::SipHasher24<'static>,
                                { #page_size },
                            >,
                            [u8; 8],
                        >,
                    >,
                >,
            >
        ))
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        let page_ty = F::page().ty().unwrap();
        let page_size = F::Page::size();
        let pages_per_bank = F::pages_per_bank();
        let peripheral_ident: proc_macro2::TokenStream =
            self.peripheral.ident().unwrap().parse().unwrap();
        let peripheral_ty = self.peripheral.ty().unwrap();

        Some(quote::quote! {
            let flash_ctrl_read_buf = kernel::static_init!(
                [u8; #page_size],
                [0; #page_size]
            );
            let page_buffer = kernel::static_init!(
                #page_ty,
                #page_ty::default()
            );

            let mux_flash = components::flash::FlashMuxComponent::new(&#peripheral_ident).finalize(
                components::flash_mux_component_static!(#peripheral_ty),
            );

            // SipHash
            let sip_hash = kernel::static_init!(
                capsules_extra::sip_hash::SipHasher24,
                capsules_extra::sip_hash::SipHasher24::new()
            );
            kernel::deferred_call::DeferredCallClient::register(sip_hash);

            // TicKV
            let tickv = components::tickv::TicKVComponent::new(
                sip_hash,
                mux_flash,                                     // Flash controller
                #pages_per_bank - 1, // Region offset (End of Bank0/Use Bank1)
                // Region Size
                #pages_per_bank * #page_size,
                flash_ctrl_read_buf, // Buffer used internally in TicKV
                page_buffer,         // Buffer used with the flash controller
            )
            .finalize(components::tickv_component_static!(
                #peripheral_ty,
                capsules_extra::sip_hash::SipHasher24,
                { #page_size }
            ));
            kernel::hil::flash::HasClient::set_client(&peripherals.flash_ctrl, mux_flash);
            kernel::hil::hasher::Hasher::set_client(sip_hash, tickv);

            let kv_store = components::kv::TicKVKVStoreComponent::new(tickv).finalize(
                components::tickv_kv_store_component_static!(
                    capsules_extra::tickv::TicKVSystem<
                        capsules_core::virtualizers::virtual_flash::FlashUser<
                            #peripheral_ty,
                        >,
                        capsules_extra::sip_hash::SipHasher24<'static>,
                        { #page_size },
                    >,
                    capsules_extra::tickv::TicKVKeyType,
                ),
            );

            let kv_store_permissions = components::kv::KVStorePermissionsComponent::new(kv_store).finalize(
                components::kv_store_permissions_component_static!(
                    capsules_extra::tickv_kv_store::TicKVKVStore<
                        capsules_extra::tickv::TicKVSystem<
                            capsules_core::virtualizers::virtual_flash::FlashUser<
                                #peripheral_ty,
                            >,
                            capsules_extra::sip_hash::SipHasher24<'static>,
                            { #page_size },
                        >,
                        capsules_extra::tickv::TicKVKeyType,
                    >
                ),
            );

            let mux_kv = components::kv::KVPermissionsMuxComponent::new(kv_store_permissions).finalize(
                components::kv_permissions_mux_component_static!(
                    capsules_extra::kv_store_permissions::KVStorePermissions<
                        capsules_extra::tickv_kv_store::TicKVKVStore<
                            capsules_extra::tickv::TicKVSystem<
                                capsules_core::virtualizers::virtual_flash::FlashUser<
                                    #peripheral_ty,
                                >,
                                capsules_extra::sip_hash::SipHasher24<'static>,
                                { #page_size },
                            >,
                            capsules_extra::tickv::TicKVKeyType,
                        >,
                    >
                ),
            );

            let virtual_kv_driver = components::kv::VirtualKVPermissionsComponent::new(mux_kv).finalize(
                components::virtual_kv_permissions_component_static!(
                    capsules_extra::kv_store_permissions::KVStorePermissions<
                        capsules_extra::tickv_kv_store::TicKVKVStore<
                            capsules_extra::tickv::TicKVSystem<
                                capsules_core::virtualizers::virtual_flash::FlashUser<
                                    #peripheral_ty,
                                >,
                                capsules_extra::sip_hash::SipHasher24<'static>,
                                { #page_size },
                            >,
                            capsules_extra::tickv::TicKVKeyType,
                        >,
                    >
                ),
            );


        })
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let driver_num = self.driver_num();
        let page_size = F::Page::size();
        let peripheral_ty = self.peripheral.ty().unwrap();

        Ok(quote::quote!(
            components::kv::KVDriverComponent::new(
                virtual_kv_driver,
                board_kernel,
                #driver_num
            )
            .finalize(components::kv_driver_component_static!(
                capsules_extra::virtual_kv::VirtualKVPermissions<
                    capsules_extra::kv_store_permissions::KVStorePermissions<
                        capsules_extra::tickv_kv_store::TicKVKVStore<
                            capsules_extra::tickv::TicKVSystem<
                                capsules_core::virtualizers::virtual_flash::FlashUser<#peripheral_ty>,
                                capsules_extra::sip_hash::SipHasher24<'static>,
                                { #page_size },
                            >,
                            capsules_extra::tickv::TicKVKeyType,
                        >,
                    >,
                >
            ))
        ))
    }
}

impl<F: flash::Flash> Capsule for KvDriver<F> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::kv_driver::DRIVER_NUM)
    }
}
