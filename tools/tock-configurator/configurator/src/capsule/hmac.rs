use std::rc::Rc;

use crate::menu::capsule_popup;
use crate::state::Data;
use cursive::view::Nameable;
use cursive::views::{Dialog, EditView};
use parse::peripherals::{Chip, DefaultPeripherals};

use super::ConfigMenu;
#[derive(Debug)]
pub(crate) struct HmacConfig;

impl ConfigMenu for HmacConfig {
    /// Menu for configuring the hmac capsule.
    fn config<C: Chip + 'static + serde::ser::Serialize>(
        chip: Rc<C>,
    ) -> cursive::views::LinearLayout {
        match chip.peripherals().hmac() {
            Ok(hmac_peripherals) => capsule_popup::<C, _>(crate::views::radio_group_with_null(
                Vec::from(hmac_peripherals),
                on_hmac_submit::<C>,
            )),
            Err(_) => capsule_popup::<C, _>(crate::menu::no_support("HMAC")),
        }
    }
}

/// Initialize a board configuration session based on the submitted chip.
fn on_hmac_submit<C: Chip + 'static + serde::ser::Serialize>(
    siv: &mut cursive::Cursive,
    submit: &Option<Rc<<C::Peripherals as DefaultPeripherals>::Hmac>>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        if let Some(hmac) = submit {
            siv.add_layer(buffer_len_popup::<C>(hmac.clone()));
        } else {
            data.platform.remove_hmac();
        }
    }
}

/// Menu for configuring the buffer length for the hmac.
fn buffer_len_popup<C: Chip + 'static + serde::ser::Serialize>(
    hmac: Rc<<C::Peripherals as DefaultPeripherals>::Hmac>,
) -> cursive::views::Dialog {
    let hmac_clone = hmac.clone();
    Dialog::around(
        EditView::new()
            .on_submit(move |siv, name| on_buffer_len_submit::<C>(siv, name, hmac.clone()))
            .with_name("buffer_len"),
    )
    .title("Buffer_len")
    .button("Save", move |siv| {
        let count = siv
            .call_on_name("buffer_len", |view: &mut EditView| view.get_content())
            .unwrap();
        on_buffer_len_submit::<C>(siv, &count, hmac_clone.clone());
    })
}

/// Add the details for the hmac and return to the hmac selection.
fn on_buffer_len_submit<C: Chip + 'static + serde::Serialize>(
    siv: &mut cursive::Cursive,
    name: &str,
    hmac: Rc<<C::Peripherals as DefaultPeripherals>::Hmac>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        let buffer_len = if name.is_empty() {
            Ok(16)
        } else {
            name.parse::<usize>()
        };

        if let Ok(buffer_len) = buffer_len {
            data.platform.update_hmac(hmac.clone(), buffer_len);
        }

        siv.pop_layer();
    }
}
