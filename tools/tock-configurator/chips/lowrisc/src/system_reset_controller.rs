#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct SystemResetController {}

impl SystemResetController {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for SystemResetController {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.sysreset"))
    }
}

impl parse::Component for SystemResetController {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::sysrst_ctrl::SysRstCtrl<'static>))
    }
}

impl std::fmt::Display for SystemResetController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "system_reset_controller")
    }
}

impl parse::peripherals::SystemResetController for SystemResetController {}
