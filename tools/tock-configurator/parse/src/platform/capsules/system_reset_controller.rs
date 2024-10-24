// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::{peripherals::system_reset_controller, Capsule, Component};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "system_reset_controller")]
pub struct SystemResetControllerCapsule<S: system_reset_controller::SystemResetController + 'static>
{
    peripheral: Rc<S>,
}

impl<S: system_reset_controller::SystemResetController + 'static> SystemResetControllerCapsule<S> {
    #[inline]
    pub fn get(peripheral: Rc<S>) -> Rc<Self> {
        Rc::new(Self::new(peripheral))
    }
}

impl<S: system_reset_controller::SystemResetController> Component
    for SystemResetControllerCapsule<S>
{
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        Ok(quote::quote!(
            capsules_extra::opentitan_sysrst::SystemReset<'static, #peripheral_ty>
        ))
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        Some(quote::quote! {
            #[cfg(not(feature = "qemu"))]
        })
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ident: proc_macro2::TokenStream = self.peripheral.ident()?.parse().unwrap();
        let ty = self.ty()?;
        let driver_num = self.driver_num();

        Ok(quote::quote!(
            {
                let opentitan_sysrst: &'static #ty = kernel::static_init!(
                    #ty,
                    capsules_extra::opentitan_sysrst::SystemReset::new(
                        &#peripheral_ident,
                        board_kernel.create_grant(#driver_num, &memory_allocation_cap)
                    ),
                );
                peripherals.sysreset.set_client(Some(opentitan_sysrst));
                peripherals.sysreset.enable_interrupts();
                opentitan_sysrst
            }
        ))
    }
}

impl<S: system_reset_controller::SystemResetController> Capsule
    for SystemResetControllerCapsule<S>
{
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::opentitan_sysrst::DRIVER_NUM)
    }
}
