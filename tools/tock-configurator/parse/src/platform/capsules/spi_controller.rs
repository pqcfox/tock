// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use crate::{component, spi, Capsule, Component, Ident};
use std::rc::Rc;

///  TODO: Doc this also.
#[component(curr, ident = "spi_controller")]
pub struct SpiController<S: spi::Spi> {
    mux_spi: Rc<spi::MuxSpi<S>>,
}

impl<S: spi::Spi + 'static> SpiController<S> {
    #[inline]
    pub fn get(peripheral: Rc<S>) -> Rc<Self> {
        Rc::new(Self::new(Rc::new(spi::MuxSpi::new(peripheral))))
    }
}

impl<S: spi::Spi + 'static> Component for SpiController<S> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.mux_spi.peripheral().ty()?;
        Ok(quote::quote!(
            capsules_core::spi_controller::Spi<
                'static,
                capsules_core::virtualizers::virtual_spi::VirtualSpiMasterDevice<
                    'static,
                    #peripheral_ty,
                >,
            >
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.mux_spi.peripheral().ty()?;
        let inner_ident: proc_macro2::TokenStream = self.mux_spi.ident()?.parse().unwrap();
        let driver_num = self.driver_num();

        Ok(quote::quote! {
            components::spi::SpiSyscallComponent::new(
                board_kernel,
                #inner_ident,
                0,
                #driver_num,
            )
            .finalize(components::spi_syscall_component_static!(#peripheral_ty))
        })
    }

    fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
        Some(vec![self.mux_spi.clone()])
    }
}

impl<S: spi::Spi + 'static> Capsule for SpiController<S> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_core::spi_controller::DRIVER_NUM)
    }
}
