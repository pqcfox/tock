// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

//! Not fully supported yet.

use parse::constants::PERIPHERALS;
use parse::peripheral;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum FlashType {
    Flash0,
}

#[derive(Debug)]
#[peripheral(serde, ident = "flash")]
pub struct Flash(FlashType);

impl parse::Component for Flash {
}

impl parse::Flash for Flash {
    type Page = parse::NoSupport;

    fn page() -> Self::Page {
        parse::NoSupport {}
    }

    fn pages_per_bank() -> proc_macro2::TokenStream {
        unimplemented!()
    }
}

impl std::fmt::Display for Flash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "flash")
    }
}
