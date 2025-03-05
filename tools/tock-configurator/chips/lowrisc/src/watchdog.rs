// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::peripherals::{Peripheral, NO_PARAM};

#[derive(Debug)]
#[parse::component(serde, ident = "watchdog")]
pub struct Watchdog;

impl parse::Component for Watchdog {
    fn ty(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::aon_timer::AonTimer<'static>))
    }

    fn init_expr(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote! {
            &peripherals.watchdog.as_ref().unwrap()
        })
    }

    fn trace_dependencies(&self, peripherals: &mut dyn parse::component::ConfigPeripherals) {
        peripherals.require(Peripheral::Watchdog as usize, NO_PARAM);
    }
}
