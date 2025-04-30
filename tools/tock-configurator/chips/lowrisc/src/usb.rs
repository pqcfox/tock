// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::peripherals::{Peripheral, NO_PARAM};

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct Usb {}

impl Usb {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for Usb {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.usb.as_ref().unwrap()"))
    }
}

impl parse::Component for Usb {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::usb::Usb<'static>))
    }

    fn trace_dependencies(&self, peripherals: &mut dyn parse::component::ConfigPeripherals) {
        peripherals.require(Peripheral::Usb as usize, NO_PARAM);
    }
}

impl std::fmt::Display for Usb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "usb")
    }
}

impl parse::peripherals::Usb for Usb {
    fn maximum_packet_size() -> proc_macro2::TokenStream {
        quote::quote!({ lowrisc::usb::MAXIMUM_PACKET_SIZE.get() })
    }
}
