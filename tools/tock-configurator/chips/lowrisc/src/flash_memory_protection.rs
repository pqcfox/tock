// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct FlashMemoryProtectionConfiguration {}

impl FlashMemoryProtectionConfiguration {
    pub(super) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for FlashMemoryProtectionConfiguration {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("flash_memory_protection_configuration"))
    }
}

impl parse::Component for FlashMemoryProtectionConfiguration {
    fn init_expr(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(get_flash_memory_protection_configuration()))
    }
}
