// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use parse::peripherals::attestation::Attestation;
use parse::peripherals::flash::Flash;
use parse::Ident;
use parse::{Component, Error};
use std::rc::Rc;

/// OpenTitan Earlgrey attestation backend
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EarlgreyAttestation<F: Flash + 'static> {
    flash: Rc<F>,
}

impl<F: Flash + 'static> EarlgreyAttestation<F> {
    pub fn new(flash: Rc<F>) -> Self {
        EarlgreyAttestation { flash }
    }
}

impl<F: Flash + 'static> parse::Ident for EarlgreyAttestation<F> {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals_attestation"))
    }
}

impl<F: Flash + 'static> Component for EarlgreyAttestation<F> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
        Some(vec![self.flash.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, Error> {
        let flash_ty = self.flash.ty()?;
        Ok(quote::quote!(earlgrey::attestation::Attestation<'static, #flash_ty>))
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        let page_ty = F::page().ty().unwrap();

        Some(quote::quote!(
            let raw_flash_ctrl_page = kernel::static_init!(
                #page_ty,
                #page_ty::default(),
            );
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, Error> {
        let ty = self.ty()?;
        let flash_identifier: proc_macro2::TokenStream = self.flash.ident()?.parse().unwrap();

        Ok(quote::quote!(kernel::static_init!(
            #ty,
            earlgrey::attestation::Attestation::new(
                #flash_identifier,
                raw_flash_ctrl_page,
            )
        )))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let flash_identifier: proc_macro2::TokenStream = self.flash.ident().ok()?.parse().unwrap();
        let earlgrey_attestation_identifier: proc_macro2::TokenStream =
            self.ident().ok()?.parse().unwrap();

        Some(quote::quote!(
            use kernel::hil::flash::HasInfoClient;
            #flash_identifier.set_info_client(#earlgrey_attestation_identifier);
        ))
    }
}

impl<F: Flash + 'static> std::fmt::Display for EarlgreyAttestation<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "earlgrey_attestation")
    }
}

impl<F: Flash + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static> Attestation
    for EarlgreyAttestation<F>
{
    type CertificateReader = F;
}
