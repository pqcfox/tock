#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct Pattgen {}

impl Pattgen {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for Pattgen {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.pattgen"))
    }
}

impl parse::Component for Pattgen {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::pattgen::PattGen<'static>))
    }
}

impl std::fmt::Display for Pattgen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pattgen")
    }
}

impl parse::peripherals::Pattgen for Pattgen {}
