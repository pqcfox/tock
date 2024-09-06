use std::rc::Rc;

use crate::menu::capsule_popup;
use crate::state::Data;
use parse::peripherals::{Chip, DefaultPeripherals};

/// Menu for configuring the Pattgen capsule.
pub fn config<C: Chip + 'static + serde::Serialize>(
    chip: Rc<C>,
    choice: Option<
        Rc<<<C as parse::peripherals::Chip>::Peripherals as DefaultPeripherals>::Pattgen>,
    >,
) -> cursive::views::LinearLayout {
    match choice {
        None => config_unknown(chip),
        Some(inner) => match chip.peripherals().pattgen() {
            Ok(pattgen_peripherals) => {
                capsule_popup::<C, _>(crate::views::radio_group_with_null_known(
                    Vec::from(pattgen_peripherals),
                    on_pattgen_submit::<C>,
                    inner,
                ))
            }
            Err(_) => capsule_popup::<C, _>(crate::menu::no_support("PATTERN GENERATOR")),
        },
    }
}

fn config_unknown<C: Chip + 'static + serde::ser::Serialize>(
    chip: Rc<C>,
) -> cursive::views::LinearLayout {
    match chip.peripherals().pattgen() {
        Ok(pattgen_peripherals) => capsule_popup::<C, _>(crate::views::radio_group_with_null(
            Vec::from(pattgen_peripherals),
            on_pattgen_submit::<C>,
        )),
        Err(_) => capsule_popup::<C, _>(crate::menu::no_support("PATTERN GENERATOR")),
    }
}

/// Configure a PatternGenerator based on the submitted pattgen.
fn on_pattgen_submit<C: Chip + 'static + serde::ser::Serialize>(
    siv: &mut cursive::Cursive,
    submit: &Option<Rc<<C::Peripherals as DefaultPeripherals>::Pattgen>>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        if let Some(pattgen) = submit {
            data.platform.update_pattgen(pattgen.clone());
        } else {
            data.platform.remove_pattgen();
        }
    }
}
