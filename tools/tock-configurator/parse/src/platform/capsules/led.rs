// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use crate::peripherals::gpio;
use crate::{Capsule, Component, Ident};

use std::rc::Rc;

/// Types of LEDs. In OxidOS, these are the structs that actually
/// wrap the low level Pin driver.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
pub enum LedType {
    /// LEDs in which on is when GPIO is high.
    LedHigh,
    /// LEDs in which on is when GPIO is low.
    LedLow,
}

impl Ident for LedType {
    fn ident(&self) -> Result<String, crate::Error> {
        Ok(String::from("LedType"))
    }
}

impl Component for LedType {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        match self {
            LedType::LedHigh => Ok(quote::quote!(kernel::hil::led::LedHigh)),
            LedType::LedLow => Ok(quote::quote!(kernel::hil::led::LedLow)),
        }
    }
}

/// The [`Led`] capsule can be configured through the GPIO pins that are used by the capsule and
/// the type of LED that they're configured as.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Led<G: gpio::Gpio> {
    /// The type of LED the pins are wrapped in.
    inner: LedType,
    /// Pins that are used by the capsule.
    pins: Vec<G::PinId>,
}

impl<G: gpio::Gpio> Led<G> {
    /// Create a new [`Led`] instance.
    pub fn new(inner: LedType, pins: Vec<G::PinId>) -> Self {
        Led { inner, pins }
    }

    pub fn add_pin(&mut self, pin: G::PinId) {
        self.pins.push(pin);
    }

    pub fn add_pins(&mut self, pins: &mut Vec<G::PinId>) {
        self.pins.append(pins);
    }

    pub fn set_inner(&mut self, inner: LedType) {
        self.inner = inner;
    }

    pub fn is_empty(&self) -> bool {
        self.pins.is_empty()
    }
}

impl<G: gpio::Gpio> Led<G> {
    #[inline]
    pub fn get(led_type: LedType, pins: Vec<G::PinId>) -> Rc<Self> {
        Rc::new(Self::new(led_type, pins))
    }

    pub fn led_type(&self) -> proc_macro2::TokenStream {
        let base_led_type_ty = self.inner.ty().unwrap();
        let pin_type = self.pins[0].ty().unwrap();

        quote::quote!(
            #base_led_type_ty<'static, #pin_type>
        )
    }
}

impl<G: gpio::Gpio + 'static> Ident for Led<G> {
    fn ident(&self) -> Result<String, crate::Error> {
        Ok(String::from("led"))
    }
}

impl<G: gpio::Gpio + 'static> Component for Led<G> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let led_type_ty = self.led_type();
        let count = self.pins.len();
        Ok(quote::quote!(
            capsules_core::led::LedDriver<
                'static,
                #led_type_ty,
                #count,
            >
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let led_type_ty = self.led_type();
        let base_led_type_ty = self.inner.ty()?;

        let pin_idents: Vec<proc_macro2::TokenStream> = self
            .pins
            .iter()
            .map(|pin| pin.ident().unwrap().parse().unwrap())
            .collect();

        let pin_maps: Vec<proc_macro2::TokenStream> = pin_idents
            .iter()
            .map(|pin| quote::quote!(#base_led_type_ty::new(&#pin)))
            .collect();

        Ok(quote::quote!(components::led::LedsComponent::new()
            .finalize(components::led_component_static!(
                #led_type_ty,
                #(#pin_maps,)*
            ))))
    }
}

impl<G: gpio::Gpio + 'static> Capsule for Led<G> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_core::led::DRIVER_NUM)
    }
}
