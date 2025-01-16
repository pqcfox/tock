// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024

use std::rc::Rc;

use crate::menu::capsule_popup;
use crate::state::Data;
use parse::peripherals::{Chip, DefaultPeripherals};

/// Menu for configuring the Attestation capsule.
pub fn config<C: Chip + 'static + serde::Serialize>(
    chip: Rc<C>,
    choice: Option<
        Rc<<<C as parse::peripherals::Chip>::Peripherals as DefaultPeripherals>::Attestation>,
    >,
) -> cursive::views::LinearLayout {
    match choice {
        None => config_unknown(chip),
        Some(inner) => match chip.peripherals().attestation() {
            Ok(attestation_peripherals) => {
                capsule_popup::<C, _>(crate::views::radio_group_with_null_known(
                    Vec::from(attestation_peripherals),
                    on_attestation_submit::<C>,
                    inner,
                ))
            }
            Err(_) => capsule_popup::<C, _>(crate::menu::no_support("ATTESTATION")),
        },
    }
}

fn config_unknown<C: Chip + 'static + serde::ser::Serialize>(
    chip: Rc<C>,
) -> cursive::views::LinearLayout {
    match chip.peripherals().attestation() {
        Ok(attestation_peripherals) => capsule_popup::<C, _>(crate::views::radio_group_with_null(
            Vec::from(attestation_peripherals),
            on_attestation_submit::<C>,
        )),
        Err(_) => capsule_popup::<C, _>(crate::menu::no_support("ATTESTATION")),
    }
}

/// Configure a Attestation based on the submitted attestation.
fn on_attestation_submit<C: Chip + 'static + serde::ser::Serialize>(
    siv: &mut cursive::Cursive,
    submit: &Option<Rc<<C::Peripherals as DefaultPeripherals>::Attestation>>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        if let Some(attestation) = submit {
            data.platform.update_attestation(attestation.clone());
        } else {
            data.platform.remove_attestation();
        }
    }
}
