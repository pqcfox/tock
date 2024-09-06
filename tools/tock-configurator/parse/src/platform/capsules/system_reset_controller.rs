use crate::{peripherals::system_reset_controller, Capsule, Component, Ident as _};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "system_reset_controller")]
pub struct SystemResetControllerCapsule<S: system_reset_controller::SystemResetController + 'static> {
    peripheral: Rc<S>,
}

impl<S: system_reset_controller::SystemResetController + 'static> SystemResetControllerCapsule<S> {
    #[inline]
    pub fn get(peripheral: Rc<S>) -> Rc<Self> {
        Rc::new(Self::new(peripheral))
    }
}

impl<S: system_reset_controller::SystemResetController> Component for SystemResetControllerCapsule<S> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        Ok(quote::quote!(
            capsules_extra::opentitan_sysrst::SystemReset<'static, #peripheral_ty>
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        let peripheral_ident: proc_macro2::TokenStream = self.peripheral.ident()?.parse().unwrap();
        let ty = self.ty()?;
        let driver_num = self.driver_num();

        Ok(quote::quote!(
            kernel::static_init!(
                #ty,
                capsules_extra::opentitan_sysrst::SystemReset::new(
                    &#peripheral_ident,
                    board_kernel.create_grant(#driver_num, &memory_allocation_cap)
                ),
            )
        ))
    }
}

impl<S: system_reset_controller::SystemResetController> Capsule for SystemResetControllerCapsule<S> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::opentitan_sysrst::DRIVER_NUM)
    }
}
