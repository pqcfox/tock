// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::peripherals::{
    EarlgreyPeripheralConfig, Peripheral, FLASH_CTRL_CONFIG_DATA, FLASH_CTRL_CONFIG_INFO,
};
use std::cell::RefCell;
use std::rc::Rc;

pub struct FlashPage;

impl parse::Ident for FlashPage {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("raw_flash_ctrl_page"))
    }
}

impl parse::Component for FlashPage {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::flash_ctrl::RawFlashCtrlPage))
    }
}

impl parse::flash::Page for FlashPage {
    fn size() -> proc_macro2::TokenStream {
        quote::quote!(earlgrey::flash_ctrl::EARLGREY_PAGE_SIZE)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, PartialEq, Clone)]
pub struct FlashCtrl {
    #[serde(skip)]
    peripherals: Rc<RefCell<EarlgreyPeripheralConfig>>,
}

impl FlashCtrl {
    pub(crate) fn new(peripherals: Rc<RefCell<EarlgreyPeripheralConfig>>) -> Self {
        Self { peripherals }
    }
}

impl parse::Ident for FlashCtrl {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.flash_ctrl.as_ref().unwrap()"))
    }
}

impl parse::Component for FlashCtrl {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::flash_ctrl::FlashCtrl<'static>))
    }

    fn trace_dependencies(&self, peripherals: &mut dyn parse::component::ConfigPeripherals) {
        // TODO: Disambiguate between requiring data / info muxes.
        peripherals.require(Peripheral::FlashCtrl as usize, FLASH_CTRL_CONFIG_DATA);
        peripherals.require(Peripheral::FlashCtrl as usize, FLASH_CTRL_CONFIG_INFO);
    }
}

impl std::fmt::Display for FlashCtrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "flash_ctrl")
    }
}

impl parse::peripherals::Flash for FlashCtrl {
    type Page = FlashPage;

    fn page() -> Self::Page {
        FlashPage {}
    }

    fn pages_per_bank() -> proc_macro2::TokenStream {
        quote::quote!(earlgrey::flash_ctrl::DATA_PAGES_PER_BANK)
    }
}
