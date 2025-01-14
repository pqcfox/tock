// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use crate::platform::peripherals::gpio;
use crate::{component, Capsule, Component, Ident};

use std::rc::Rc;

#[component(curr, ident = "gpio")]
pub struct GPIO<G: gpio::Gpio> {
    pins: Vec<G::PinId>,
}

impl<G: gpio::Gpio + 'static> GPIO<G> {
    #[inline]
    pub fn get(pins: Vec<G::PinId>) -> Rc<Self> {
        Rc::new(Self::new(pins))
    }
}

impl<G: gpio::Gpio + 'static> Component for GPIO<G> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let pin_ty = self.pins[0].ty()?;
        Ok(quote::quote!(
            capsules_core::gpio::GPIO<'static, #pin_ty>
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let driver_num = self.driver_num();
        let pin_ty = self.pins[0].ty()?;

        let pin_idents: Vec<proc_macro2::TokenStream> = self
            .pins
            .iter()
            .map(|pin| pin.ident().unwrap().parse().unwrap())
            .collect();

        let pin_maps: Vec<proc_macro2::TokenStream> = pin_idents
            .iter()
            .enumerate()
            .map(|(index, pin_ident)| quote::quote!(#index => &#pin_ident))
            .collect();

        Ok(quote::quote!(
            components::gpio::GpioComponent::new(
                board_kernel,
                #driver_num,
                components::gpio_component_helper!(
                    #pin_ty,
                    #(#pin_maps,)*
                )
            )
            .finalize(components::gpio_component_static!(
                #pin_ty
            ))
        ))
    }

    fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
        None
    }
}

impl<G: gpio::Gpio + 'static> Capsule for GPIO<G> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_core::gpio::DRIVER_NUM)
    }
}
