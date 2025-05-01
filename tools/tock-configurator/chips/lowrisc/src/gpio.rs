// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::peripherals::Peripheral;
use std::rc::Rc;

pub const GPIO_PINS: usize = 32;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum PinId {
    Pin0 = 0,
    Pin1,
    Pin2,
    Pin3,
    Pin4,
    Pin5,
    Pin6,
    Pin7,
    Pin8,
    Pin9,
    Pin10,
    Pin11,
    Pin12,
    Pin13,
    Pin14,
    Pin15,
    Pin16,
    Pin17,
    Pin18,
    Pin19,
    Pin20,
    Pin21,
    Pin22,
    Pin23,
    Pin24,
    Pin25,
    Pin26,
    Pin27,
    Pin28,
    Pin29,
    Pin30,
    Pin31,
}

impl std::fmt::Display for PinId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pin{}", *self as usize)
    }
}

impl parse::Ident for PinId {
    fn ident(&self) -> Result<String, parse::Error> {
        let index = *self as usize;
        Ok(format!(
            "peripherals.gpio_port.as_ref().unwrap()[{}]",
            index
        ))
    }
}

impl parse::Component for PinId {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(
            earlgrey::gpio::GpioPin<'static, earlgrey::pinmux::PadConfig>
        ))
    }

    fn trace_dependencies(&self, peripherals: &mut dyn parse::component::ConfigPeripherals) {
        peripherals.require(Peripheral::GpioPort as usize, *self as usize)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct GpioPort {}

impl GpioPort {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for GpioPort {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.gpio_port.as_ref().unwrap()"))
    }
}

impl parse::Component for GpioPort {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::gpio::Port<'static>))
    }
}

impl std::fmt::Display for GpioPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "gpio_port")
    }
}

impl parse::peripherals::Gpio for GpioPort {
    type PinId = PinId;

    fn pins(&self) -> Option<std::rc::Rc<[Self::PinId]>> {
        Some(Rc::new([
            PinId::Pin0,
            PinId::Pin1,
            PinId::Pin2,
            PinId::Pin3,
            PinId::Pin4,
            PinId::Pin5,
            PinId::Pin6,
            PinId::Pin7,
            PinId::Pin8,
            PinId::Pin9,
            PinId::Pin10,
            PinId::Pin11,
            PinId::Pin12,
            PinId::Pin13,
            PinId::Pin14,
            PinId::Pin15,
            PinId::Pin16,
            PinId::Pin17,
            PinId::Pin18,
            PinId::Pin19,
            PinId::Pin20,
            PinId::Pin21,
            PinId::Pin22,
            PinId::Pin23,
            PinId::Pin24,
            PinId::Pin25,
            PinId::Pin26,
            PinId::Pin27,
            PinId::Pin28,
            PinId::Pin29,
            PinId::Pin30,
            PinId::Pin31,
        ]))
    }
}
