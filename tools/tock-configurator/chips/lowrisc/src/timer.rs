// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use parse::Ident as _;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct RvTimer {}

impl RvTimer {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for RvTimer {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.timer"))
    }
}

impl parse::Component for RvTimer {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::timer::RvTimer<'static>))
    }

    fn after_init(&self) -> Option<parse::proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        Some(quote::quote!(#ident.setup()))
    }
}

impl parse::peripherals::Timer for RvTimer {
    fn frequency(&self) -> usize {
        0
    }
}

impl std::fmt::Display for RvTimer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "timer")
    }
}
