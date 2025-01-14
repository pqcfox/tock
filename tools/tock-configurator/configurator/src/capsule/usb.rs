// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use std::rc::Rc;

use crate::menu::capsule_popup;
use crate::state::Data;
use parse::peripherals::{Chip, DefaultPeripherals};

/// Menu for configuring the Usb capsule.
pub fn config<C: Chip + 'static + serde::Serialize>(
    chip: Rc<C>,
    choice: Option<Rc<<<C as parse::peripherals::Chip>::Peripherals as DefaultPeripherals>::Usb>>,
) -> cursive::views::LinearLayout {
    match choice {
        None => config_unknown(chip),
        Some(inner) => match chip.peripherals().usb() {
            Ok(usb_peripherals) => {
                capsule_popup::<C, _>(crate::views::radio_group_with_null_known(
                    Vec::from(usb_peripherals),
                    on_usb_submit::<C>,
                    inner,
                ))
            }
            Err(_) => capsule_popup::<C, _>(crate::menu::no_support("USB")),
        },
    }
}

fn config_unknown<C: Chip + 'static + serde::ser::Serialize>(
    chip: Rc<C>,
) -> cursive::views::LinearLayout {
    match chip.peripherals().usb() {
        Ok(usb_peripherals) => capsule_popup::<C, _>(crate::views::radio_group_with_null(
            Vec::from(usb_peripherals),
            on_usb_submit::<C>,
        )),
        Err(_) => capsule_popup::<C, _>(crate::menu::no_support("USB")),
    }
}

/// Configure a Usb based on the submitted usb.
fn on_usb_submit<C: Chip + 'static + serde::ser::Serialize>(
    siv: &mut cursive::Cursive,
    submit: &Option<Rc<<C::Peripherals as DefaultPeripherals>::Usb>>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        if let Some(usb) = submit {
            data.platform.update_usb(usb.clone());
        } else {
            data.platform.remove_usb();
        }
    }
}
