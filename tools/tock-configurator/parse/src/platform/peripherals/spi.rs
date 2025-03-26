// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use std::rc::Rc;

use super::NoSupport;
use crate::{component, Component};

pub trait Spi: crate::Component + std::fmt::Debug + std::fmt::Display {}

#[component(curr, ident = "mux_spi")]
pub struct MuxSpi<S: Spi> {
    pub(crate) peripheral: Rc<S>,
}

impl<S: Spi> MuxSpi<S> {
    pub fn peripheral(&self) -> Rc<S> {
        self.peripheral.clone()
    }
}

impl<S: Spi + 'static> Component for MuxSpi<S> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn crate::Component>>> {
        Some(vec![self.peripheral.clone()])
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_type = self.peripheral.ty()?;
        let peripheral_identifier: proc_macro2::TokenStream =
            self.peripheral.ident()?.parse().unwrap();
        Ok(quote::quote!(
            components::spi::SpiMuxComponent::new(&#peripheral_identifier).finalize(
                components::spi_mux_component_static!(#peripheral_type)
            )
        ))
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_type = self.peripheral.ty()?;
        Ok(quote::quote!(components::spi::SpiMuxComponent<#peripheral_type>))
    }
}

impl Spi for NoSupport {}
