// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use std::rc::Rc;

use crate::menu::capsule_popup;
use crate::state::Data;
use parse::peripherals::{Chip, DefaultPeripherals};

/// Menu for configuring the ResetManager capsule.
pub fn config<C: Chip + 'static + serde::Serialize>(
    chip: Rc<C>,
    choice: Option<
        Rc<<<C as parse::peripherals::Chip>::Peripherals as DefaultPeripherals>::ResetManager>,
    >,
) -> cursive::views::LinearLayout {
    match choice {
        None => config_unknown(chip),
        Some(inner) => match chip.peripherals().reset_manager() {
            Ok(reset_manager_peripherals) => {
                capsule_popup::<C, _>(crate::views::radio_group_with_null_known(
                    Vec::from(reset_manager_peripherals),
                    on_reset_manager_submit::<C>,
                    inner,
                ))
            }
            Err(_) => capsule_popup::<C, _>(crate::menu::no_support("RESET MANAGER")),
        },
    }
}

fn config_unknown<C: Chip + 'static + serde::ser::Serialize>(
    chip: Rc<C>,
) -> cursive::views::LinearLayout {
    match chip.peripherals().reset_manager() {
        Ok(reset_manager_peripherals) => {
            capsule_popup::<C, _>(crate::views::radio_group_with_null(
                Vec::from(reset_manager_peripherals),
                on_reset_manager_submit::<C>,
            ))
        }
        Err(_) => capsule_popup::<C, _>(crate::menu::no_support("RESET MANAGER")),
    }
}

/// Configure a ResetManager based on the submitted reset_manager.
fn on_reset_manager_submit<C: Chip + 'static + serde::ser::Serialize>(
    siv: &mut cursive::Cursive,
    submit: &Option<Rc<<C::Peripherals as DefaultPeripherals>::ResetManager>>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        if let Some(reset_manager) = submit {
            data.platform.update_reset_manager(reset_manager.clone());
        } else {
            data.platform.remove_reset_manager();
        }
    }
}
