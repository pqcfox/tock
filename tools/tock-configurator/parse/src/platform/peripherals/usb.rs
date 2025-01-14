// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use super::NoSupport;

pub trait Usb: crate::Component + std::fmt::Debug + std::fmt::Display {
    fn maximum_packet_size() -> proc_macro2::TokenStream;
}

impl Usb for NoSupport {
    fn maximum_packet_size() -> proc_macro2::TokenStream {
        unimplemented!()
    }
}
