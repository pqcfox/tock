use crate::{peripherals::uart, Capsule, Ident};

use std::rc::Rc;

#[parse_macros::component(curr, ident = "lldb")]
pub struct Lldb<U: uart::Uart> {
    pub(crate) mux_uart: Rc<uart::MuxUart<U>>,
}

impl<U: uart::Uart + 'static> Lldb<U> {
    pub fn get(mux_uart: Rc<uart::MuxUart<U>>) -> Rc<Self> {
        Rc::new(Self::new(mux_uart))
    }
}

impl<U: uart::Uart + 'static> crate::Component for Lldb<U> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn crate::Component>>> {
        Some(vec![self.mux_uart.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        Ok(quote::quote!(
            capsules_core::low_level_debug::LowLevelDebug<
                'static,
                capsules_core::virtualizers::virtual_uart::UartDevice<'static>,
            >
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let mux_uart = quote::format_ident!("{}", self.mux_uart.ident()?);
        let driver_num = self.driver_num();

        Ok(quote::quote! {
            components::lldb::LowLevelDebugComponent::new(
                board_kernel,
                #driver_num,
                #mux_uart,
            )
            .finalize(components::low_level_debug_component_static!())
        })
    }
}

impl<U: uart::Uart + 'static> crate::Capsule for Lldb<U> {
    fn driver_num(&self) -> proc_macro2::TokenStream {
        quote::quote! {
            capsules_core::low_level_debug::DRIVER_NUM
        }
    }
}
