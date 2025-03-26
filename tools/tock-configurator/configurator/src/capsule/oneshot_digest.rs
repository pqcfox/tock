// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use std::rc::Rc;

use crate::menu::capsule_popup;
use crate::state::Data;
use parse::peripherals::{Chip, DefaultPeripherals};

/// Menu for configuring the OneshotDigest capsule.
pub fn config<C: Chip + 'static + serde::Serialize>(
    chip: Rc<C>,
    choice: Option<
        Rc<<<C as parse::peripherals::Chip>::Peripherals as DefaultPeripherals>::OneshotDigest>,
    >,
) -> cursive::views::LinearLayout {
    match choice {
        None => config_unknown(chip),
        Some(inner) => match chip.peripherals().oneshot_digest() {
            Ok(oneshot_digest_peripherals) => {
                capsule_popup::<C, _>(crate::views::radio_group_with_null_known(
                    Vec::from(oneshot_digest_peripherals),
                    on_oneshot_digest_submit::<C>,
                    inner,
                ))
            }
            Err(_) => capsule_popup::<C, _>(crate::menu::no_support("ONESHOT DIGEST")),
        },
    }
}

fn config_unknown<C: Chip + 'static + serde::ser::Serialize>(
    chip: Rc<C>,
) -> cursive::views::LinearLayout {
    match chip.peripherals().oneshot_digest() {
        Ok(oneshot_digest_peripherals) => {
            capsule_popup::<C, _>(crate::views::radio_group_with_null(
                Vec::from(oneshot_digest_peripherals),
                on_oneshot_digest_submit::<C>,
            ))
        }
        Err(_) => capsule_popup::<C, _>(crate::menu::no_support("ONESHOT DIGEST")),
    }
}

/// Configure a OneshotDigest based on the submitted oneshot_digest.
fn on_oneshot_digest_submit<C: Chip + 'static + serde::ser::Serialize>(
    siv: &mut cursive::Cursive,
    submit: &Option<Rc<<C::Peripherals as DefaultPeripherals>::OneshotDigest>>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        if let Some(oneshot_digest) = submit {
            data.platform.update_oneshot_digest(oneshot_digest.clone());
        } else {
            data.platform.remove_oneshot_digest();
        }
    }
}
