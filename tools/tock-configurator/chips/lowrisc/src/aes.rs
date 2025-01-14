// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct Aes {}

impl Aes {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for Aes {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.aes"))
    }
}

impl parse::Component for Aes {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::aes::Aes<'static>))
    }
}

impl std::fmt::Display for Aes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "aes")
    }
}

impl parse::peripherals::Aes for Aes {}
