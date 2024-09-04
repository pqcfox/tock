#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct Uart {}

impl Uart {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for Uart {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.uart0"))
    }
}

impl parse::Component for Uart {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::uart::Uart<'static>))
    }
}

impl std::fmt::Display for Uart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "uart")
    }
}

impl parse::peripherals::Uart for Uart {}
