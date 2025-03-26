// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use super::peripherals::timer;
use crate::{Component, Ident as _};
use std::rc::Rc;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SchedulerTimer<T: timer::Timer + 'static> {
    virtual_mux_alarm: Rc<timer::VirtualMuxAlarm<T>>,
}

impl<T: timer::Timer + 'static> crate::Ident for SchedulerTimer<T> {
    fn ident(&self) -> Result<String, crate::error::Error> {
        Ok(String::from("scheduler_timer"))
    }
}

impl<T: timer::Timer + 'static> SchedulerTimer<T> {
    pub fn new(virtual_mux_alarm: Rc<timer::VirtualMuxAlarm<T>>) -> Rc<Self> {
        Rc::new(Self { virtual_mux_alarm })
    }
}

impl<T: timer::Timer + 'static> Component for SchedulerTimer<T> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
        Some(vec![self.virtual_mux_alarm.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let virtual_mux_alarm_ty = self.virtual_mux_alarm.ty()?;

        Ok(quote::quote!(
            kernel::platform::scheduler_timer::VirtualSchedulerTimer<#virtual_mux_alarm_ty>
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let ty = self.ty()?;
        let virtual_mux_alarm: proc_macro2::TokenStream =
            self.virtual_mux_alarm.ident()?.parse().unwrap();
        Ok(quote::quote!(kernel::static_init!(
            #ty,
            kernel::platform::scheduler_timer::VirtualSchedulerTimer::new(#virtual_mux_alarm),
        )))
    }
}

impl<T: timer::Timer + 'static> SchedulerTimer<T> {
    pub fn virtual_mux_alarm(&self) -> Rc<timer::VirtualMuxAlarm<T>> {
        self.virtual_mux_alarm.clone()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct DefaultSchedulerTimer;

impl DefaultSchedulerTimer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for DefaultSchedulerTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::Ident for DefaultSchedulerTimer {
    fn ident(&self) -> Result<String, crate::error::Error> {
        Ok(String::from("scheduler_timer"))
    }
}

impl Component for DefaultSchedulerTimer {
    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        Ok(quote::quote!(()))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        Ok(quote::quote!(&()))
    }
}
