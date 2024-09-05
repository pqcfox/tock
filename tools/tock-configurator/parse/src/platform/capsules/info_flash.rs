use crate::{peripherals::flash, Capsule, Component, Ident as _};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "info_flash")]
pub struct InfoFlash<F: flash::Flash + 'static> {
    peripheral: Rc<F>,
}

impl<F: flash::Flash + 'static> InfoFlash<F> {
    #[inline]
    pub fn get(peripheral: Rc<F>) -> Rc<Self> {
        Rc::new(Self::new(peripheral))
    }
}

impl<F: flash::Flash> Component for InfoFlash<F> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        Ok(quote::quote!(capsules_extra::info_flash::InfoFlash<'static, #peripheral_ty>))
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        let page_ty = F::page().ty().unwrap();

        Some(quote::quote!(
            let raw_flash_ctrl_page = kernel::static_init!(
                #page_ty,
                #page_ty::default(),
            );
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let ty = self.ty()?;
        let peripheral_identifier: proc_macro2::TokenStream = self.peripheral.ident()?.parse().unwrap();
        let driver_number = self.driver_num();

        Ok(quote::quote!(
            kernel::static_init!(
                #ty,
                capsules_extra::info_flash::InfoFlash::new(
                    &#peripheral_identifier,
                    board_kernel.create_grant(
                        #driver_number,
                        &memory_allocation_cap,
                    ),
                    raw_flash_ctrl_page,
                )
            )
        ))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        let peripheral_identifier: proc_macro2::TokenStream = self.peripheral.ident().unwrap().parse().unwrap();

        Some(quote::quote! {
            use kernel::hil::flash::HasInfoClient;
            #peripheral_identifier.set_info_client(#ident);
        })
    }
}

impl<F: flash::Flash> Capsule for InfoFlash<F> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::info_flash::DRIVER_NUMBER)
    }
}
