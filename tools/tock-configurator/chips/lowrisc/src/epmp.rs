#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Epmp {

}

impl Epmp {
    pub fn new() -> Self {
        Self {

        }
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
    fn init_expr(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote! {
            // Set up memory protection immediately after setting the trap handler, to
            // ensure that much of the board initialization routine runs with ePMP
            // protection.
            earlgrey::epmp::EarlGreyEPMP::new_debug(
                earlgrey::epmp::FlashRegion(
                    rv32i::pmp::NAPOTRegionSpec::new(
                        core::ptr::addr_of!(_sflash),
                        core::ptr::addr_of!(_eflash) as usize - core::ptr::addr_of!(_sflash) as usize,
                    )
                    .unwrap(),
                ),
                earlgrey::epmp::RAMRegion(
                    rv32i::pmp::NAPOTRegionSpec::new(
                        core::ptr::addr_of!(_ssram),
                        core::ptr::addr_of!(_esram) as usize - core::ptr::addr_of!(_ssram) as usize,
                    )
                    .unwrap(),
                ),
                earlgrey::epmp::MMIORegion(
                    rv32i::pmp::NAPOTRegionSpec::new(
                        0x40000000 as *const u8, // start
                        0x10000000,              // size
                    )
                    .unwrap(),
                ),
                earlgrey::epmp::KernelTextRegion(
                    rv32i::pmp::TORRegionSpec::new(
                        core::ptr::addr_of!(_stext),
                        core::ptr::addr_of!(_etext),
                    )
                    .unwrap(),
                ),
                // RV Debug Manager memory region (required for JTAG debugging).
                // This access can be disabled by changing the EarlGreyEPMP type
                // parameter `EPMPDebugConfig` to `EPMPDebugDisable`, in which case
                // this expects to be passed a unit (`()`) type.
                earlgrey::epmp::RVDMRegion(
                    rv32i::pmp::NAPOTRegionSpec::new(
                        0x00010000 as *const u8, // start
                        0x00001000,              // size
                    )
                    .unwrap(),
                ),
            )
            .unwrap();
        })
    }
}
