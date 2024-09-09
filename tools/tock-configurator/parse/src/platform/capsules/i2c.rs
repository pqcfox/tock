// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use crate::platform::peripherals::i2c;
use crate::{component, Capsule, Component};

use std::rc::Rc;

#[component(curr, ident = "i2c_master")]
pub struct I2CMasterDriver<I: i2c::I2c> {
    peripheral: Rc<I>,
}

impl<I: i2c::I2c + 'static> I2CMasterDriver<I> {
    #[inline]
    pub fn get(peripheral: Rc<I>) -> Rc<Self> {
        Rc::new(Self::new(peripheral))
    }
}

impl<I: i2c::I2c + 'static> Component for I2CMasterDriver<I> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        Ok(quote::quote!(
            capsules_core::i2c_master::I2CMasterDriver<'static, #peripheral_ty>
        ))
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        Some(quote::quote!(
            let i2c_master_buffer = kernel::static_init!(
                [u8; capsules_core::i2c_master::BUFFER_LENGTH],
                [0; capsules_core::i2c_master::BUFFER_LENGTH],
            );
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ident: proc_macro2::TokenStream = self.peripheral.ident()?.parse().unwrap();
        let ty = self.ty()?;
        let driver_num = self.driver_num();
        Ok(quote::quote!(kernel::static_init!(
            #ty,
            capsules_core::i2c_master::I2CMasterDriver::new(
                &#peripheral_ident,
                i2c_master_buffer,
                board_kernel.create_grant(
                    #driver_num,
                    &memory_allocation_cap,
                ),
            )
        )))
    }

    fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
        None
    }
}

impl<I: i2c::I2c + 'static> Capsule for I2CMasterDriver<I> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_core::i2c_master::DRIVER_NUM)
    }
}
