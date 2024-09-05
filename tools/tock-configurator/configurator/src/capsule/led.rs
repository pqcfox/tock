use crate::items::ToMenuItem;
use crate::menu::{capsule_popup, no_support, pin_list_disabled};
use crate::state::{on_exit_submit, on_quit_submit, Data, PinFunction};
use crate::views;
use cursive::views::{Checkbox, ListChild, ListView};
use cursive::Cursive;
use parse::peripherals::{Chip, DefaultPeripherals, Gpio};
use std::cell::RefCell;
use std::rc::Rc;

use super::ConfigMenu;

#[derive(Debug)]
pub(crate) struct LedConfig;

impl ConfigMenu for LedConfig {
    /// Menu for configuring the led capsule.
    fn config<C: Chip + 'static + serde::ser::Serialize>(
        chip: Rc<C>,
    ) -> cursive::views::LinearLayout {
        match chip.peripherals().gpio() {
            Ok(list) => capsule_popup::<C, _>(views::select_menu(
                Vec::from(list)
                    .into_iter()
                    .map(|elem| elem.to_menu_item())
                    .collect(),
                |siv, submit| {
                    crate::state::on_gpio_submit::<C, _>(siv, submit.clone(), led_type_popup::<C>)
                },
            )),
            Err(_) => capsule_popup::<C, _>(no_support("GPIO")),
        }
    }
}

fn led_type_popup<C: Chip + 'static + serde::ser::Serialize>(
    gpio: Rc<<C::Peripherals as DefaultPeripherals>::Gpio>,
) -> cursive::views::LinearLayout {
    capsule_popup::<C, _>(views::select_menu(
        vec![
            ("LedHigh", parse::capsules::led::LedType::LedHigh),
            ("LedLow", parse::capsules::led::LedType::LedLow),
        ],
        move |siv, choice| on_led_type_submit::<C>(siv, gpio.clone(), choice.clone()),
    ))
}

fn led_pins_popup<C: Chip + 'static + serde::ser::Serialize>(
    gpio: Rc<<C::Peripherals as DefaultPeripherals>::Gpio>,
    pin_list: Vec<(
        <<<C as Chip>::Peripherals as DefaultPeripherals>::Gpio as Gpio>::PinId,
        PinFunction,
    )>,
    led_type: parse::capsules::led::LedType,
) -> cursive::views::LinearLayout {
    let view = pin_list_disabled::<C>(pin_list, PinFunction::Led, "led_pins");
    let gpio_clone = Rc::clone(&gpio);
    let led_type_clone = led_type.clone();
    crate::menu::checkbox_popup(
        view,
        move |siv: &mut Cursive| {
            on_led_pin_submit::<C>(siv, Rc::clone(&gpio), led_type.clone(), false)
        },
        move |siv: &mut Cursive| {
            on_led_pin_submit::<C>(siv, Rc::clone(&gpio_clone), led_type_clone.clone(), true)
        },
    )
}

fn on_led_type_submit<C: Chip + 'static + serde::Serialize>(
    siv: &mut cursive::Cursive,
    gpio: Rc<<C::Peripherals as DefaultPeripherals>::Gpio>,
    led_type: parse::capsules::led::LedType,
) {
    if let Some(data) = siv.user_data::<Data<C>>() {
        //data.current_view_type = ViewType::LedPins;
        let pin_list = data.gpio(&gpio).unwrap().pins().clone();
        crate::state::push_layer::<_, C>(siv, led_pins_popup::<C>(gpio, pin_list, led_type));
    }
}

fn on_led_pin_submit<C: Chip + 'static + serde::Serialize>(
    siv: &mut cursive::Cursive,
    gpio: Rc<<C::Peripherals as DefaultPeripherals>::Gpio>,
    led_type: parse::capsules::led::LedType,
    quit: bool,
) {
    let pin_names = RefCell::new(Vec::new());
    siv.call_on_name("led_pins", |list: &mut ListView| {
        for child in list.children() {
            if let ListChild::Row(label, view) = child {
                if let Some(check) = view.downcast_ref::<Checkbox>() {
                    if check.is_checked() {
                        pin_names.borrow_mut().push(label.clone());
                    }
                }
            }
        }
    });

    if let Some(data) = siv.user_data::<Data<C>>() {
        let mut selected_pins = Vec::new();
        if let Some(pin_list) = gpio.pins() {
            for pin in pin_list.as_ref() {
                if pin_names.borrow().contains(&format!("{:?}", pin)) {
                    selected_pins.push(*pin);
                }
            }
        }

        let mut unselected_pins = Vec::new();
        for (pin, pin_function) in data.gpio(&gpio).unwrap().pins() {
            if *pin_function == PinFunction::Led && !selected_pins.contains(pin) {
                unselected_pins.push(*pin);
            }
        }

        for pin in selected_pins.iter() {
            data.change_pin_status(Rc::clone(&gpio), *pin, PinFunction::Led);
        }

        for pin in unselected_pins.iter() {
            data.change_pin_status(Rc::clone(&gpio), *pin, PinFunction::None);
        }

        if selected_pins.is_empty() {
            data.platform.remove_led();
        } else {
            data.platform.update_led(led_type, selected_pins);
        }
    }

    if quit {
        on_quit_submit::<C>(siv);
    } else {
        on_exit_submit::<C>(siv);
    }
}
