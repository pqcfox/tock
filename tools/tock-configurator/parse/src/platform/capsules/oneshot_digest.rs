// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::{peripherals::oneshot_digest, Capsule, Component};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "oneshot_digest")]
pub struct OneshotDigestCapsule<P: oneshot_digest::OneshotDigest + 'static> {
    peripheral: Rc<P>,
}

impl<P: oneshot_digest::OneshotDigest + 'static> OneshotDigestCapsule<P> {
    #[inline]
    pub fn get(peripheral: Rc<P>) -> Rc<Self> {
        Rc::new(Self::new(peripheral))
    }
}

impl<P: oneshot_digest::OneshotDigest> Component for OneshotDigestCapsule<P> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn crate::Component>>> {
        Some(vec![self.peripheral.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        Ok(quote::quote!(
            capsules_extra::oneshot_digest::OneshotDigest<#peripheral_ty>
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ident: proc_macro2::TokenStream = self.peripheral.ident()?.parse().unwrap();
        let ty = self.ty()?;
        let driver_num = self.driver_num();

        Ok(quote::quote!(kernel::static_init!(
            #ty,
            capsules_extra::oneshot_digest::OneshotDigest::new(
                #peripheral_ident,
                board_kernel.create_grant(#driver_num, &memory_allocation_cap)
            ),
        )))
    }
}

impl<P: oneshot_digest::OneshotDigest> Capsule for OneshotDigestCapsule<P> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::oneshot_digest::DRIVER_NUM)
    }
}
