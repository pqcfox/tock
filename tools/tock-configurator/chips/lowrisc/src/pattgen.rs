// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::peripherals::{Peripheral, NO_PARAM};

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct Pattgen {}

impl Pattgen {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for Pattgen {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.pattgen.as_ref().unwrap()"))
    }
}

impl parse::Component for Pattgen {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::pattgen::PattGen<'static>))
    }

    fn trace_dependencies(&self, peripherals: &mut dyn parse::component::ConfigPeripherals) {
        peripherals.require(Peripheral::Pattgen as usize, NO_PARAM);
    }
}

impl std::fmt::Display for Pattgen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pattgen")
    }
}

impl parse::peripherals::Pattgen for Pattgen {}
