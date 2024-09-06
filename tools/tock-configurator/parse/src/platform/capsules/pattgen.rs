use crate::{peripherals::pattgen, Capsule, Component, Ident as _};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "pattgen")]
pub struct PattgenCapsule<P: pattgen::Pattgen + 'static> {
    peripheral: Rc<P>,
}

impl<P: pattgen::Pattgen + 'static> PattgenCapsule<P> {
    #[inline]
    pub fn get(peripheral: Rc<P>) -> Rc<Self> {
        Rc::new(Self::new(peripheral))
    }
}

impl<P: pattgen::Pattgen> Component for PattgenCapsule<P> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        Ok(quote::quote!(
            capsules_extra::pattgen::PattGen<'static, #peripheral_ty>
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
                capsules_extra::pattgen::PattGen::new(
                    &#peripheral_ident,
                    board_kernel.create_grant(#driver_num, &memory_allocation_cap)
                ),
            )
        ))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        let peripheral_ident: proc_macro2::TokenStream = self.peripheral.ident().unwrap().parse().unwrap();

        Some(quote::quote!(
            kernel::hil::pattgen::PattGen::set_client(&#peripheral_ident, #ident);
        ))
    }
}

impl<P: pattgen::Pattgen> Capsule for PattgenCapsule<P> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::pattgen::DRIVER_NUM)
    }
}
