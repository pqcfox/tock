#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct AlertHandler {}

impl AlertHandler {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for AlertHandler {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.alert_handler"))
    }
}

impl parse::Component for AlertHandler {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::sysrst_ctrl::SysRstCtrl<'static>))
    }
}

impl std::fmt::Display for AlertHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "alert_handler")
    }
}

impl parse::peripherals::AlertHandler for AlertHandler {}
