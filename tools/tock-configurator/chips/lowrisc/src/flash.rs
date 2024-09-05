pub struct FlashPage;

impl parse::Ident for FlashPage {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("raw_flash_ctrl_page"))
    }
}

impl parse::Component for FlashPage {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::flash_ctrl::RawFlashCtrlPage))
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct FlashCtrl {}

impl FlashCtrl {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for FlashCtrl {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.flash_ctrl"))
    }
}

impl parse::Component for FlashCtrl {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::flash_ctrl::FlashCtrl<'static>))
    }
}

impl std::fmt::Display for FlashCtrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "flash_ctrl")
    }
}

impl parse::peripherals::Flash for FlashCtrl {
    type Page = FlashPage;

    fn page() -> Self::Page {
        FlashPage {}
    }
}
