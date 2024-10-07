#[derive(Debug)]
#[parse::component(serde, ident = "watchdog")]
pub struct Watchdog;

impl parse::Component for Watchdog {
    fn ty(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(
            lowrisc::aon_timer::AonTimer<'static>
        ))
    }

    fn init_expr(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote! {
            &peripherals.watchdog
        })
    }
}
