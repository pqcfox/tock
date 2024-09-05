#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct Hmac {}

impl Hmac {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for Hmac {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.hmac"))
    }
}

impl parse::Component for Hmac {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::hmac::Hmac<'static>))
    }
}

impl std::fmt::Display for Hmac {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hmac")
    }
}

impl parse::peripherals::Hmac for Hmac {}
