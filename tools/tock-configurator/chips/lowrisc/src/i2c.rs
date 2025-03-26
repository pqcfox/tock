// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct I2c {}

impl I2c {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for I2c {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.i2c0"))
    }
}

impl parse::Component for I2c {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::i2c::I2c<'static>))
    }
}

impl std::fmt::Display for I2c {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "i2c0")
    }
}

impl parse::peripherals::I2c for I2c {}
