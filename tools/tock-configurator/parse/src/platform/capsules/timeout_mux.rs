// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::component::Ident;
use crate::{Component, MuxAlarm, Timer};
use std::rc::Rc;

/// Timeout multiplexer
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TimeoutMux<T: crate::Timer + 'static> {
    mux_alarm: std::rc::Rc<MuxAlarm<T>>,
}

impl<T: Timer + 'static> TimeoutMux<T> {
    pub fn new(mux_alarm: std::rc::Rc<MuxAlarm<T>>) -> Self {
        Self { mux_alarm }
    }

    pub fn mux_alarm(&self) -> std::rc::Rc<MuxAlarm<T>> {
        self.mux_alarm.clone()
    }
}

impl<T: crate::Timer + 'static> crate::Ident for TimeoutMux<T> {
    fn ident(&self) -> Result<String, crate::Error> {
        Ok(String::from("timeout_mux"))
    }
}

impl<T: crate::Timer + 'static> crate::Component for TimeoutMux<T> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
        Some(vec![self.mux_alarm.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let timer_ty = self.mux_alarm.timer().ty()?;
        Ok(quote::quote!(
            capsules_core::virtualizers::timeout_mux::TimeoutMux<'static, #timer_ty, lowrisc::ffi::cryptolib::mux::OtbnOperation<'static, #timer_ty>>
        ))
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        let timer_ty: proc_macro2::TokenStream = self.mux_alarm.timer().ty().unwrap();
        let mux_alarm_ident: proc_macro2::TokenStream =
            self.mux_alarm.ident().unwrap().parse().unwrap();

        Some(quote::quote!(
            use capsules_core::virtualizers::virtual_alarm::VirtualMuxAlarm;
            use capsules_core::virtualizers::virtual_timer::{MuxTimer, VirtualTimer};


            let virtual_alarm_user: &'static VirtualMuxAlarm<'static, #timer_ty> = kernel::static_init!(
                VirtualMuxAlarm<'static, #timer_ty>,
                VirtualMuxAlarm::new(#mux_alarm_ident)
            );
            virtual_alarm_user.setup();
            let mux_timer: &'static MuxTimer<'static, #timer_ty> = kernel::static_init!(
                MuxTimer<'static, #timer_ty>,
                MuxTimer::new(virtual_alarm_user)
            );
            kernel::hil::time::Alarm::set_alarm_client(virtual_alarm_user, mux_timer);
            let otbn_timer: &'static VirtualTimer<'static, #timer_ty> =
                kernel::static_init!(VirtualTimer<'static, #timer_ty>, VirtualTimer::new(mux_timer),);
            otbn_timer.setup();
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let ty = self.ty()?;
        Ok(quote::quote!(kernel::static_init!(
            #ty,
            capsules_core::virtualizers::timeout_mux::TimeoutMux::new(
                otbn_timer,
                lowrisc::ffi::cryptolib::mux::OTBN_TIMEOUT_MUX_CHECK_FREQ
            ),
        )))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        Some(quote::quote!(
            kernel::hil::time::Timer::set_timer_client(otbn_timer, #ident);
        ))
    }
}

impl<T: Timer + 'static> std::fmt::Display for TimeoutMux<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "timeout_mux({})", self.mux_alarm)
    }
}
