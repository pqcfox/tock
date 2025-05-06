// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::{peripherals::flash, Capsule, Component, Ident as _};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "info_flash_user", serde)]
#[derive(Debug)]
pub struct InfoFlashUser<F: flash::Flash + 'static> {
    peripheral: Rc<F>,
}

impl<F: flash::Flash + 'static> InfoFlashUser<F> {
    #[inline]
    pub fn get(peripheral: Rc<F>) -> Rc<InfoFlashUser<F>> {
        Rc::new(Self::new(peripheral))
    }
}

impl<F: flash::Flash + 'static> Component for InfoFlashUser<F> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn crate::Component>>> {
        Some(vec![self.peripheral.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        Ok(
            quote::quote!(capsules_core::virtualizers::virtual_flash::InfoFlashUser<'static, #peripheral_ty>),
        )
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        let peripheral_ident: proc_macro2::TokenStream =
            self.peripheral.ident().unwrap().parse().unwrap();
        let peripheral_ty = self.peripheral.ty().unwrap();
        Some(quote::quote! {
            let mux_info_flash = components::flash::InfoFlashMuxComponent::new(#peripheral_ident)
                .finalize(components::info_flash_mux_component_static!(
                    #peripheral_ty
                ));
        })
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;

        Ok(quote::quote! {
            components::flash::InfoFlashUserComponent::new(mux_info_flash)
                .finalize(components::info_flash_user_component_static!(
                    #peripheral_ty
                ))
        })
    }
}

impl<F: flash::Flash + 'static> std::fmt::Display for InfoFlashUser<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InfoFlashUser({})", &self.peripheral)
    }
}

// Implement `Flash` for `InfoFlashUser` so it can be used as a type parameter
// to other peripherals.
impl<F: flash::Flash + 'static> flash::Flash for InfoFlashUser<F> {
    type Page = F::Page;

    fn page() -> Self::Page {
        F::page()
    }

    fn pages_per_bank() -> proc_macro2::TokenStream {
        F::pages_per_bank()
    }
}

#[component(curr, ident = "info_flash")]
pub struct InfoFlash<F: flash::Flash + 'static> {
    _peripheral: Rc<F>,
    info_flash_user: Rc<InfoFlashUser<F>>,
}

impl<F: flash::Flash + 'static> InfoFlash<F> {
    #[inline]
    pub fn get(peripheral: Rc<F>) -> Rc<Self> {
        let info_flash_user = Rc::new(InfoFlashUser::new(peripheral.clone()));
        Rc::new(Self::new(peripheral, info_flash_user))
    }
}

impl<F: flash::Flash> Component for InfoFlash<F> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn crate::Component>>> {
        Some(vec![self.info_flash_user.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let info_flash_user_ty = self.info_flash_user.ty()?;
        Ok(quote::quote!(capsules_extra::info_flash::InfoFlash<'static, #info_flash_user_ty>))
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        let page_ty = F::page().ty().unwrap();

        Some(quote::quote!(
            let raw_flash_ctrl_page = kernel::static_init!(
                #page_ty,
                #page_ty::default(),
            );
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let ty = self.ty()?;
        let info_flash_user_identifier: proc_macro2::TokenStream =
            self.info_flash_user.ident()?.parse().unwrap();
        let driver_number = self.driver_num();

        Ok(quote::quote!(kernel::static_init!(
            #ty,
            capsules_extra::info_flash::InfoFlash::new(
                #info_flash_user_identifier,
                board_kernel.create_grant(
                    #driver_number,
                    &memory_allocation_cap,
                ),
                raw_flash_ctrl_page,
            )
        )))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        let info_flash_user_identifier: proc_macro2::TokenStream =
            self.info_flash_user.ident().unwrap().parse().unwrap();

        Some(quote::quote! {
            HasInfoClient::set_info_client(#info_flash_user_identifier, #ident);
        })
    }
}

impl<F: flash::Flash> Capsule for InfoFlash<F> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::info_flash::DRIVER_NUMBER)
    }
}
