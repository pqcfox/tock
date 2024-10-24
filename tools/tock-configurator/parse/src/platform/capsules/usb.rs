// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use crate::{peripherals::usb, Capsule, Component, Ident as _};
use parse_macros::component;
use std::rc::Rc;

#[component(curr, ident = "usb_client")]
struct UsbClient<U: usb::Usb + 'static> {
    peripheral: Rc<U>,
}

impl<U: usb::Usb + 'static> UsbClient<U> {
    fn get(peripheral: Rc<U>) -> Rc<Self> {
        Rc::new(Self::new(peripheral))
    }

    fn peripheral(&self) -> Rc<U> {
        self.peripheral.clone()
    }
}

impl<U: usb::Usb> Component for UsbClient<U> {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.peripheral.ty()?;
        let maximum_packet_size = U::maximum_packet_size();
        Ok(quote::quote!(
            capsules_extra::usb::usb_user2::UsbClient<
                'static,
                #peripheral_ty,
                #maximum_packet_size,
            >
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let ty = self.ty()?;
        let peripheral_ident: proc_macro2::TokenStream = self.peripheral.ident()?.parse().unwrap();

        Ok(quote::quote! {
            kernel::static_init!(
                #ty,
                capsules_extra::usb::usb_user2::UsbClient::new(
                    &#peripheral_ident
                )
            )
        })
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let peripheral_ident: proc_macro2::TokenStream =
            self.peripheral.ident().unwrap().parse().unwrap();
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();

        Some(quote::quote! {
            use kernel::hil::usb::UsbController;
            #peripheral_ident.set_client(#ident);
        })
    }
}

#[component(curr, ident = "usb")]
pub struct UsbCapsule<U: usb::Usb + 'static> {
    usb_client: Rc<UsbClient<U>>,
}

impl<U: usb::Usb + 'static> UsbCapsule<U> {
    #[inline]
    pub fn get(peripheral: Rc<U>) -> Rc<Self> {
        Rc::new(Self::new(UsbClient::get(peripheral)))
    }
}

impl<U: usb::Usb> Component for UsbCapsule<U> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
        Some(vec![self.usb_client.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let peripheral_ty = self.usb_client.peripheral().ty()?;
        let maximum_packet_size = U::maximum_packet_size();

        Ok(quote::quote!(
            capsules_extra::usb::usb_user2::UsbSyscallDriver<
                'static,
                #peripheral_ty,
                #maximum_packet_size,
            >
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let ty = self.ty()?;
        let driver_num = self.driver_num();
        let usb_client_ident: proc_macro2::TokenStream = self.usb_client.ident()?.parse().unwrap();

        Ok(quote::quote!(kernel::static_init!(
            #ty,
            capsules_extra::usb::usb_user2::UsbSyscallDriver::new(
                #usb_client_ident,
                board_kernel.create_grant(
                    #driver_num,
                    &memory_allocation_cap,
                )
            )
        )))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();

        Some(quote::quote!(#ident.init();))
    }
}

impl<U: usb::Usb> Capsule for UsbCapsule<U> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote!(capsules_extra::usb::usb_user2::DRIVER_NUM)
    }
}
