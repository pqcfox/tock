use std::rc::Rc;

use crate::menu::capsule_popup;
use crate::state::Data;
use parse::peripherals::{Chip, DefaultPeripherals};

/// Menu for configuring the SystemResetController capsule.
pub fn config<C: Chip + 'static + serde::Serialize>(
    chip: Rc<C>,
    choice: Option<
        Rc<<<C as parse::peripherals::Chip>::Peripherals as DefaultPeripherals>::SystemResetController>,
    >,
) -> cursive::views::LinearLayout {
    match choice {
        None => config_unknown(chip),
        Some(inner) => match chip.peripherals().system_reset_controller() {
            Ok(system_reset_controller_peripherals) => {
                capsule_popup::<C, _>(crate::views::radio_group_with_null_known(
                    Vec::from(system_reset_controller_peripherals),
                    on_system_reset_controller_submit::<C>,
                    inner,
                ))
            }
            Err(_) => capsule_popup::<C, _>(crate::menu::no_support("SYSTEM RESET CONTROLLER")),
        },
    }
}

fn config_unknown<C: Chip + 'static + serde::ser::Serialize>(
    chip: Rc<C>,
) -> cursive::views::LinearLayout {
    match chip.peripherals().system_reset_controller() {
        Ok(system_reset_controller_peripherals) => capsule_popup::<C, _>(crate::views::radio_group_with_null(
            Vec::from(system_reset_controller_peripherals),
            on_system_reset_controller_submit::<C>,
        )),
        Err(_) => capsule_popup::<C, _>(crate::menu::no_support("SYSTEM RESET CONTROLLER")),
    }
}

/// Configure a PatternGenerator based on the submitted system_reset_controller.
fn on_system_reset_controller_submit<C: Chip + 'static + serde::ser::Serialize>(
    siv: &mut cursive::Cursive,
    submit: &Option<Rc<<C::Peripherals as DefaultPeripherals>::SystemResetController>>,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        if let Some(system_reset_controller) = submit {
            data.platform.update_system_reset_controller(system_reset_controller.clone());
        } else {
            data.platform.remove_system_reset_controller();
        }
    }
}
