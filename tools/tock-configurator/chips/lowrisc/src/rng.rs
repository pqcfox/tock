#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct CsRng {}

impl CsRng {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for CsRng {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.rng"))
    }
}

impl parse::Component for CsRng {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(lowrisc::csrng::CsRng<'static>))
    }
}

impl std::fmt::Display for CsRng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rng")
    }
}

impl parse::peripherals::Rng for CsRng {}
