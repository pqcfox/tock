// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use crate::menu::capsule_popup;
use crate::state::Data;
use parse::peripherals::Chip;

const PERIPHERAL: &str = "IPC";

/// Menu for configuring the IPC capsule.
pub fn config<C: Chip + 'static + serde::Serialize>(
    previous_state: Option<()>,
) -> cursive::views::LinearLayout {
    match previous_state {
        None => config_none::<C>(),
        Some(()) => capsule_popup::<C, _>(crate::views::radio_group_with_null_known(
            vec![PERIPHERAL],
            on_ipc_submit::<C>,
            PERIPHERAL,
        )),
    }
}

/// Menu for configuring the Alarm capsule when none was configured before.
fn config_none<C: Chip + 'static + serde::ser::Serialize>() -> cursive::views::LinearLayout {
    capsule_popup::<C, _>(crate::views::radio_group_with_null(
        vec![PERIPHERAL],
        on_ipc_submit::<C>,
    ))
}

/// Configure an IPC capsule.
fn on_ipc_submit<C: Chip + 'static + serde::ser::Serialize>(
    siv: &mut cursive::Cursive,
    submit: &Option<&'static str>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        match submit {
            Some(_) => data.platform.update_ipc(),
            None => data.platform.remove_ipc(),
        }
    }
}
