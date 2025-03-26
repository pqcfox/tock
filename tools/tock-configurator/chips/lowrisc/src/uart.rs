// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum Uart {
    Uart0,
}

impl PartialEq<Uart> for Uart {
    fn eq(&self, _other: &Uart) -> bool {
        true
    }
}

impl Uart {
    pub(crate) fn new() -> Self {
        Uart::Uart0
    }
}

impl parse::Ident for Uart {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.uart0"))
    }
}

impl parse::Component for Uart {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::uart::Uart<'static>))
    }
}

impl std::fmt::Display for Uart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "uart")
    }
}

impl parse::peripherals::Uart for Uart {}
