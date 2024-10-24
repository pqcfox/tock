// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::{peripherals::reset_manager, Capsule, Component, Ident as _};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "reset_manager")]
pub struct ResetManagerCapsule<A: reset_manager::ResetManager + 'static> {
    peripheral: Rc<A>,
}

impl<A: reset_manager::ResetManager + 'static> ResetManagerCapsule<A> {
    #[inline]
    pub fn get(peripheral: Rc<A>) -> Rc<Self> {
        Rc::new(Self::new(peripheral))
    }
}

impl<A: reset_manager::ResetManager> Component for ResetManagerCapsule<A> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;

        Ok(quote::quote!(
            capsules_extra::reset_manager::ResetManager<'static, #peripheral_ty>
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let ty = self.ty()?;
        let peripheral_ident: proc_macro2::TokenStream =
            self.peripheral.ident().unwrap().parse().unwrap();
        let driver_num = self.driver_num();

        Ok(quote::quote!(kernel::static_init!(
            #ty,
            capsules_extra::reset_manager::ResetManager::new(
                &#peripheral_ident,
                board_kernel.create_grant(#driver_num, &memory_allocation_cap),
            ),
        )))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        let peripheral_ident: proc_macro2::TokenStream =
            self.peripheral.ident().unwrap().parse().unwrap();

        Some(quote::quote!(
            #[cfg(not(feature = "qemu"))]
            {
                use kernel::hil::reset_managment::ResetManagment;
                let reset_reason = #peripheral_ident
                    .reset_reason()
                    .or(earlgrey::rstmgr::RstMgr::get_rr_from_rram(&peripherals.sram_ret));
                #ident.startup();
                #ident.populate_reset_reason(reset_reason);
            }
        ))
    }
}

impl<A: reset_manager::ResetManager> Capsule for ResetManagerCapsule<A> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::reset_manager::DRIVER_NUM)
    }
}
