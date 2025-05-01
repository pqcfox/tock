// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use super::NoSupport;
use crate::{Component, Ident as _};
use quote::quote;
use std::rc::Rc;

/// A trait that applies to clocks that implement the `Timer`-related traits defined in
/// Tock's HIL.
///
///  TODO: Maybe move to a `Peripheral` trait that implements Component?
pub trait Timer: std::fmt::Debug + PartialEq + std::fmt::Display + Component {
    /// Timer's frequency. Used for providing information in the configuration process.
    fn frequency(&self) -> usize;
}

/// Implementation for the unit type.
impl Timer for NoSupport {
    fn frequency(&self) -> usize {
        0
    }
}

/// Virtual multiplexed alarm. The configurator must resort to this type in case
/// the same alarm may need to be used for multiple capsules/kernel resources.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct VirtualMuxAlarm<T: Timer + 'static> {
    mux_alarm: Rc<MuxAlarm<T>>,
}

impl<T: Timer> VirtualMuxAlarm<T> {
    pub fn new(mux_alarm: Rc<MuxAlarm<T>>) -> Self {
        Self { mux_alarm }
    }

    pub fn mux_alarm(&self) -> Rc<MuxAlarm<T>> {
        self.mux_alarm.clone()
    }
}

impl<T: Timer + 'static> crate::Ident for VirtualMuxAlarm<T> {
    fn ident(&self) -> Result<String, crate::error::Error> {
        Ok(String::from("virtual_mux_alarm"))
    }
}

impl<T: Timer + 'static> Component for VirtualMuxAlarm<T> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn Component>>> {
        Some(vec![self.mux_alarm.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let timer_ty = self.mux_alarm.peripheral.ty()?;

        Ok(
            quote!(capsules_core::virtualizers::virtual_alarm::VirtualMuxAlarm<
            'static,
            #timer_ty,
        >),
        )
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let ty = self.ty()?;
        let mux_alarm_ident: proc_macro2::TokenStream = self.mux_alarm.ident()?.parse().unwrap();
        Ok(quote::quote!(kernel::static_init!(
            #ty,
            capsules_core::virtualizers::virtual_alarm::VirtualMuxAlarm::new(#mux_alarm_ident)
        )))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();

        Some(quote::quote!(#ident.setup();))
    }
}

/// Multiplexed alarm. The configurator must resort to this type in case
/// the same alarm may need to be used for multiple capsules/kernel resources.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MuxAlarm<T: Timer + 'static> {
    pub(crate) peripheral: Rc<T>,
}

//  TODO: Remove these clones...
impl<T: Timer> MuxAlarm<T> {
    pub fn timer(&self) -> Rc<T> {
        self.peripheral.clone()
    }
}

impl<T: Timer + 'static> MuxAlarm<T> {
    pub fn new(peripheral: Rc<T>) -> Self {
        Self { peripheral }
    }

    pub fn insert_get(peripheral: Rc<T>, visited: &mut Vec<Rc<dyn Component>>) -> Rc<Self> {
        for node in visited.iter() {
            if let Ok(mux_alarm) = node.clone().downcast::<MuxAlarm<T>>() {
                if mux_alarm.timer() == peripheral {
                    return mux_alarm;
                }
            }
        }

        let mux_alarm = Rc::new(MuxAlarm::new(peripheral));
        visited.push(mux_alarm.clone() as Rc<dyn Component>);

        mux_alarm
    }
}

impl<T: Timer> crate::Ident for MuxAlarm<T> {
    fn ident(&self) -> Result<String, crate::error::Error> {
        Ok(String::from("mux_alarm"))
    }
}

impl<T: Timer> crate::Component for MuxAlarm<T> {
    fn dependencies(&self) -> Option<Vec<Rc<dyn crate::Component>>> {
        Some(vec![self.peripheral.clone()])
    }

    fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let (timer_ident, timer_type): (proc_macro2::TokenStream, _) = (
            self.peripheral.ident()?.parse().unwrap(),
            self.peripheral.ty()?,
        );
        Ok(quote! {
        components::alarm::AlarmMuxComponent::new(#timer_ident)
        .finalize(components::alarm_mux_component_static!(#timer_type))})
    }

    fn before_init(&self) -> Option<proc_macro2::TokenStream> {
        let timer_ident: proc_macro2::TokenStream =
            self.peripheral.ident().unwrap().parse().unwrap();
        Some(quote! {
            let __timer = #timer_ident;
        })
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
        let timer_type = self.peripheral.ty()?;

        Ok(quote!(components::alarm::AlarmMuxComponent::new(__timer)
            .finalize(components::alarm_mux_component_static!(
                    #timer_type
            ))))
    }

    fn after_init(&self) -> Option<proc_macro2::TokenStream> {
        let ident: proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();

        Some(quote!(
            #[cfg(test)]
            {
                ALARM = Some(#ident);
            }
        ))
    }
}

impl<T: Timer + 'static> std::fmt::Display for MuxAlarm<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mux_alarm({})", self.peripheral)
    }
}
