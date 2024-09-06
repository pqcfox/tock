use crate::{peripherals::alert_handler, Capsule, Component, Ident as _};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "alert_handler")]
pub struct AlertHandlerCapsule<A: alert_handler::AlertHandler + 'static> {
    peripheral: Rc<A>,
}

impl<A: alert_handler::AlertHandler + 'static> AlertHandlerCapsule<A> {
    #[inline]
    pub fn get(peripheral: Rc<A>) -> Rc<Self> {
        Rc::new(Self::new(peripheral))
    }
}

impl<A: alert_handler::AlertHandler> Component for AlertHandlerCapsule<A> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        Ok(quote::quote!(
            capsules_extra::opentitan_alerthandler::AlertHandlerCapsule
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let ty = self.ty()?;
        let driver_num = self.driver_num();

        Ok(quote::quote!(
            kernel::static_init!(
                #ty,
                capsules_extra::opentitan_alerthandler::AlertHandlerCapsule::new(
                    board_kernel.create_grant(#driver_num, &memory_allocation_cap)
                ),
            )
        ))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        let peripheral_ident: proc_macro2::TokenStream = self.peripheral.ident().unwrap().parse().unwrap();

        Some(quote::quote!(
            earlgrey::alert_handler::AlertHandler::set_client(&#peripheral_ident, #ident);
        ))
    }
}

impl<A: alert_handler::AlertHandler> Capsule for AlertHandlerCapsule<A> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::opentitan_alerthandler::DRIVER_NUM)
    }
}
