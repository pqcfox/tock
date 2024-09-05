use crate::{peripherals::hmac, Capsule, Component};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "hmac")]
pub struct HmacCapsule<H: hmac::Hmac + 'static> {
    peripheral: Rc<H>,
    length: usize,
}

impl<H: hmac::Hmac + 'static> HmacCapsule<H> {
    #[inline]
    pub fn get(peripheral: Rc<H>, length: usize) -> Rc<Self> {
        Rc::new(Self::new(peripheral, length))
    }
}

impl<H: hmac::Hmac> Component for HmacCapsule<H> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        let length = self.length;

        Ok(quote::quote!(
            capsules_extra::hmac::HmacDriver<'static, #peripheral_ty, #length>
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let driver_num = self.driver_num();
        let peripheral = &self.peripheral;
        let peripheral_identifier: proc_macro2::TokenStream = peripheral.ident()?.parse().unwrap();
        let peripheral_ty = peripheral.ty()?;
        let length = self.length;

        Ok(quote::quote!(
            components::hmac::HmacComponent::new(
                board_kernel,
                #driver_num,
                &#peripheral_identifier,
            )
            .finalize(components::hmac_component_static!(#peripheral_ty, #length))
        ))
    }
}

impl<H: hmac::Hmac> Capsule for HmacCapsule<H> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::hmac::DRIVER_NUM)
    }
}
