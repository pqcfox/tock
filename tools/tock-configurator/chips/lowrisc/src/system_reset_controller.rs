// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct SystemResetController {}

impl SystemResetController {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for SystemResetController {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.sysreset"))
    }
}

impl parse::Component for SystemResetController {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::sysrst_ctrl::SysRstCtrl<'static>))
    }
}

impl std::fmt::Display for SystemResetController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "system_reset_controller")
    }
}

impl parse::peripherals::SystemResetController for SystemResetController {}
