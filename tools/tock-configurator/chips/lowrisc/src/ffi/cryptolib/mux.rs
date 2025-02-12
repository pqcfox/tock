// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use parse::platform::capsules::TimeoutMux;
use parse::{Component, Error, Ident, Timer};
use std::rc::Rc;

/// Cryptolib OTBN multiplexer
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CryptolibMux<T: Timer + 'static> {
    timeout_mux: Rc<TimeoutMux<T>>,
}

impl<T: Timer + 'static> CryptolibMux<T> {
    pub fn new(timeout_mux: std::rc::Rc<TimeoutMux<T>>) -> Self {
        Self { timeout_mux }
    }

    pub fn timeout_mux(&self) -> std::rc::Rc<TimeoutMux<T>> {
        self.timeout_mux.clone()
    }
}

impl<T: Timer + 'static> parse::Ident for CryptolibMux<T> {
    fn ident(&self) -> Result<String, Error> {
        Ok(String::from("cryptolib_mux"))
    }
}

impl<T: Timer + 'static> parse::Component for CryptolibMux<T> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
        Some(vec![self.timeout_mux.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, Error> {
        let timer_ty = self.timeout_mux.mux_alarm().timer().ty()?;
        Ok(quote::quote!(lowrisc::ffi::cryptolib::mux::CryptolibMux<'static, #timer_ty>))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, Error> {
        let timeout_mux_ident: proc_macro2::TokenStream =
            self.timeout_mux.ident()?.parse().unwrap();
        let ty = self.ty()?;

        Ok(quote::quote!(kernel::static_init!(
            #ty,
            lowrisc::ffi::cryptolib::mux::CryptolibMux::new(
                earlgrey::otbn::OTBN_BASE,
                #timeout_mux_ident
            ),
        )))
    }
}

impl<T: Timer + 'static> std::fmt::Display for CryptolibMux<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cryptolib_mux({})", self.timeout_mux)
    }
}
