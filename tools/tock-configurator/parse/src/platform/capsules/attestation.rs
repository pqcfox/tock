// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::{peripherals::attestation, Capsule, Component, Ident as _};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "opentitan_attestation")]
pub struct AttestationCapsule<A: attestation::Attestation + 'static> {
    chip_specific_attestation: Rc<A>,
}

impl<A: attestation::Attestation + 'static> AttestationCapsule<A> {
    #[inline]
    pub fn get(chip_specific: Rc<A>) -> Rc<Self> {
        Rc::new(Self::new(chip_specific))
    }
}

impl<A: attestation::Attestation> Component for AttestationCapsule<A> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn crate::Component>>> {
        Some(vec![self.chip_specific_attestation.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let chip_specific_ty = self.chip_specific_attestation.ty()?;
        Ok(quote::quote!(
            capsules_extra::opentitan_attestation::Attestation<'static, #chip_specific_ty>
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let chip_specific_ident: proc_macro2::TokenStream =
            self.chip_specific_attestation.ident()?.parse().unwrap();
        let ty = self.ty()?;
        let driver_num = self.driver_num();

        Ok(quote::quote!(
            kernel::static_init!(
                #ty,
                capsules_extra::opentitan_attestation::Attestation::new(
                    #chip_specific_ident,
                    board_kernel.create_grant(#driver_num, &memory_allocation_cap)
                )
            );
        ))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        let chip_specific_ident: proc_macro2::TokenStream = self
            .chip_specific_attestation
            .ident()
            .unwrap()
            .parse()
            .unwrap();

        Some(quote::quote!(
            use kernel::hil::opentitan_attestation::CertificateReader;
            #chip_specific_ident.set_client(#ident);
        ))
    }
}

impl<A: attestation::Attestation> Capsule for AttestationCapsule<A> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::opentitan_attestation::DRIVER_NUM)
    }
}
