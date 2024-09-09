// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use std::rc::Rc;

use crate::menu::capsule_popup;
use crate::state::Data;
use cursive::view::Nameable;
use cursive::views::{Dialog, EditView};
use parse::peripherals::{Chip, DefaultPeripherals};

use super::ConfigMenu;
#[derive(Debug)]
pub(crate) struct AesConfig;

impl ConfigMenu for AesConfig {
    /// Menu for configuring the hmac capsule.
    fn config<C: Chip + 'static + serde::ser::Serialize>(
        chip: Rc<C>,
    ) -> cursive::views::LinearLayout {
        match chip.peripherals().aes() {
            Ok(aes_peripherals) => capsule_popup::<C, _>(crate::views::radio_group_with_null(
                Vec::from(aes_peripherals),
                on_aes_submit::<C>,
            )),
            Err(_) => capsule_popup::<C, _>(crate::menu::no_support("AES")),
        }
    }
}

/// Initialize a board configuration session based on the submitted chip.
fn on_aes_submit<C: Chip + 'static + serde::ser::Serialize>(
    siv: &mut cursive::Cursive,
    submit: &Option<Rc<<C::Peripherals as DefaultPeripherals>::Aes>>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        if let Some(aes) = submit {
            siv.add_layer(crypt_size_popup::<C>(aes.clone()));
        } else {
            data.platform.remove_aes();
        }
    }
}

/// Menu for configuring the crypt size for the aes.
fn crypt_size_popup<C: Chip + 'static + serde::ser::Serialize>(
    aes: Rc<<C::Peripherals as DefaultPeripherals>::Aes>,
) -> cursive::views::Dialog {
    let aes_clone = aes.clone();
    Dialog::around(
        EditView::new()
            .on_submit(move |siv, name| on_crypt_size_submit::<C>(siv, name, aes.clone()))
            .with_name("crypt_size"),
    )
    .title("Crypt_Size")
    .button("Save", move |siv| {
        let count = siv
            .call_on_name("crypt_size", |view: &mut EditView| view.get_content())
            .unwrap();
        on_crypt_size_submit::<C>(siv, &count, aes_clone.clone());
    })
}

/// Add the details for the aes and return to the aes selection.
fn on_crypt_size_submit<C: Chip + 'static + serde::Serialize>(
    siv: &mut cursive::Cursive,
    name: &str,
    aes: Rc<<C::Peripherals as DefaultPeripherals>::Aes>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        let crypt_size = if name.is_empty() {
            Ok(7)
        } else {
            name.parse::<usize>()
        };

        if let Ok(crypt_size) = crypt_size {
            data.platform.update_aes(aes.clone(), crypt_size);
        }

        siv.pop_layer();
    }
}
