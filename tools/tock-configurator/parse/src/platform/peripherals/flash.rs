// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use crate::Component;

use super::NoSupport;

pub trait Page: Component {
    fn size() -> proc_macro2::TokenStream;
}

impl Page for NoSupport {
    fn size() -> proc_macro2::TokenStream {
        unimplemented!()
    }
}

pub trait Flash: Component + std::fmt::Display {
    type Page: Page;

    fn page() -> Self::Page;
    fn pages_per_bank() -> proc_macro2::TokenStream;
}
impl Flash for NoSupport {
    type Page = NoSupport;

    fn page() -> Self::Page {
        NoSupport {}
    }

    fn pages_per_bank() -> proc_macro2::TokenStream {
        unimplemented!()
    }
}
