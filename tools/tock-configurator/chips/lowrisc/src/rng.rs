// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::peripherals::{Peripheral, NO_PARAM};

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct CsRng {}

impl CsRng {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for CsRng {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.rng.as_ref().unwrap()"))
    }
}

impl parse::Component for CsRng {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::csrng::CsRng<'static>))
    }

    fn trace_dependencies(&self, peripherals: &mut dyn parse::component::ConfigPeripherals) {
        peripherals.require(Peripheral::Rng as usize, NO_PARAM);
    }
}

impl std::fmt::Display for CsRng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rng")
    }
}

impl parse::peripherals::Rng for CsRng {}
