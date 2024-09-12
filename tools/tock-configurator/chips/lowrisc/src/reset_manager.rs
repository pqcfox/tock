#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct ResetManager {}

impl ResetManager {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for ResetManager {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.rst_mgmt"))
    }
}

impl parse::Component for ResetManager {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::rstmgr::RstMgr))
    }

}

impl std::fmt::Display for ResetManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "reset_manager")
    }
}

impl parse::peripherals::ResetManager for ResetManager {}
