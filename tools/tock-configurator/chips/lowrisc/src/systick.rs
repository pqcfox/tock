// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

#[derive(Debug)]
#[parse::component(serde, ident = "scheduler_timer")]
pub struct Systick {
    virtual_mux_alarm: Rc<lowrisc::timer::RvTimer<'static>>,
}

impl parse::Component for Systick {
    fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
        Some(vec![self.virtual_mux_alarm.clone()])
    }

    fn ty(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(
            kernel::platform::scheduler_timer::VirtualSchedulerTimer<
                capsules_core::virtualizers::virtual_alarm::VirtualMuxAlarm<'static, lowrisc::timer::RvTimer<'static>>,
            >
        ))
    }

    fn init_expr(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        let ty = self.ty()?;

        Ok(quote::quote! {
            kernel::static_init!(
                #ty,
                kernel::platform::scheduler_timer::VirtualSchedulerTimer::new(scheduler_timer_virtual_alarm),
            )
        })
    }
}
