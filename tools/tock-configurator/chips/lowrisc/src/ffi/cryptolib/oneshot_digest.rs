// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
pub struct OtCryptoOneshotDigest {}

impl OtCryptoOneshotDigest {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for OtCryptoOneshotDigest {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("otcrypto_oneshot_digest"))
    }
}

impl parse::Component for OtCryptoOneshotDigest {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        let ty = self.ty()?;

        Ok(quote::quote!(#ty))
    }
}

impl std::fmt::Display for OtCryptoOneshotDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "otcrypto_oneshot_digest")
    }
}

impl parse::peripherals::OneshotDigest for OtCryptoOneshotDigest {}
