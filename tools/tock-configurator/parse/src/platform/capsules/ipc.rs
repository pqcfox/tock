// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use crate::component::Capsule as _;

use std::rc::Rc;

#[parse_macros::component(curr, ident = "ipc")]
pub struct IPC;

impl IPC {
    pub fn get() -> Rc<Self> {
        Rc::new(Self::new())
    }
}

impl crate::Component for IPC {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        Ok(quote::quote! {
            kernel::ipc::IPC<{ NUM_PROCS as u8 }>
        })
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let driver_num = self.driver_num();
        let ty = self.ty()?;

        Ok(quote::quote! {
            kernel::static_init!(
                #ty,
                kernel::ipc::IPC::new(
                    board_kernel,
                    #driver_num,
                    &memory_allocation_cap,
                ),
            )
        })
    }
}

impl crate::Capsule for IPC {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(kernel::ipc::DRIVER_NUM)
    }
}
