#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct RvTimer {}

impl RvTimer {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for RvTimer {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.timer"))
    }
}

impl parse::Component for RvTimer {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::timer::RvTimer<'static, ChipConfig>))
    }
}

impl parse::peripherals::Timer for RvTimer {
    fn frequency(&self) -> usize {
        0
    }
}

impl std::fmt::Display for RvTimer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "timer")
    }
}
