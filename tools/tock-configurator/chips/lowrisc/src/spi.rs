// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::peripherals::{Peripheral, NO_PARAM};

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct SpiHost {}

impl SpiHost {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for SpiHost {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.spi_host0.as_ref().unwrap()"))
    }
}

impl parse::Component for SpiHost {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::spi_host::SpiHost<'static>))
    }

    fn trace_dependencies(&self, peripherals: &mut dyn parse::component::ConfigPeripherals) {
        peripherals.require(Peripheral::SpiHost0 as usize, NO_PARAM);
    }
}

impl std::fmt::Display for SpiHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "spi_host0")
    }
}

impl parse::peripherals::Spi for SpiHost {}
