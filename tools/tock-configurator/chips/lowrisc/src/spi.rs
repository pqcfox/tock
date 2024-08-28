#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct SpiHost {}

impl SpiHost {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for SpiHost {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.spi_host0"))
    }
}

impl parse::Component for SpiHost {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::spi_host::SpiHost<'static>))
    }
}

impl std::fmt::Display for SpiHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "spi_host0")
    }
}

impl parse::peripherals::Spi for SpiHost {}
