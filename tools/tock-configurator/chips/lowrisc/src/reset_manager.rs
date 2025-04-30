// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::peripherals::{Peripheral, NO_PARAM};

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct ResetManager {}

impl ResetManager {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for ResetManager {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.rst_mgmt.as_ref().unwrap()"))
    }
}

impl parse::Component for ResetManager {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::rstmgr::RstMgr))
    }

    fn trace_dependencies(&self, peripherals: &mut dyn parse::component::ConfigPeripherals) {
        peripherals.require(Peripheral::RstMgmt as usize, NO_PARAM);
    }
}

impl std::fmt::Display for ResetManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "reset_manager")
    }
}

impl parse::peripherals::ResetManager for ResetManager {}
