// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Epmp {}

impl Epmp {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Epmp {
    fn default() -> Self {
        Self::new()
    }
}

impl parse::Ident for Epmp {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("earlgrey_epmp"))
    }
}

impl parse::Component for Epmp {
    fn before_init(&self) -> Option<parse::proc_macro2::TokenStream> {
        Some(quote::quote! {
            let flash_region = earlgrey::epmp::FlashRegion(
                rv32i::pmp::NAPOTRegionSpec::new(
                    core::ptr::addr_of!(_sflash),
                    core::ptr::addr_of!(_eflash) as usize - core::ptr::addr_of!(_sflash) as usize,
                )
                .unwrap(),
            );
            let ram_region = earlgrey::epmp::RAMRegion(
                rv32i::pmp::NAPOTRegionSpec::new(
                    core::ptr::addr_of!(_ssram),
                    core::ptr::addr_of!(_esram) as usize - core::ptr::addr_of!(_ssram) as usize,
                )
                .unwrap(),
            );
            let mmio_region = earlgrey::epmp::MMIORegion(
                rv32i::pmp::NAPOTRegionSpec::new(
                    0x40000000 as *const u8, // start
                    0x10000000,              // size
                )
                .unwrap(),
            );
            let kernel_text_region = earlgrey::epmp::KernelTextRegion(
                rv32i::pmp::TORRegionSpec::new(core::ptr::addr_of!(_stext), core::ptr::addr_of!(_etext))
                    .unwrap(),
            );
            #[cfg(feature = "sival")]
        })
    }

    fn init_expr(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote! {
            earlgrey::epmp::EarlGreyEPMP::new(
                flash_region,
                ram_region,
                mmio_region,
                kernel_text_region,
            )
            .unwrap();
            #[cfg(not(feature = "sival"))]
            let earlgrey_epmp = {
                let debug_region = earlgrey::epmp::RVDMRegion(
                    rv32i::pmp::NAPOTRegionSpec::new(
                        0x00010000 as *const u8, // start
                        0x00001000,              // size
                    )
                    .unwrap(),
                );
                earlgrey::epmp::EarlGreyEPMP::new_debug(
                    flash_region,
                    ram_region,
                    mmio_region,
                    kernel_text_region,
                    debug_region,
                )
                .unwrap()
            };
        })
    }
}
