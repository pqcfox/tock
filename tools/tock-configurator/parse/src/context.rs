// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use std::error::Error;
use std::rc::Rc;

use crate::config::{Capsule, Configuration};
use crate::{
    AlarmDriver, Console, Led, MuxAlarm, MuxUart, RngCapsule,
    TemperatureCapsule, SpiCapsule, I2CMasterDriver, GPIO, HmacCapsule,
    InfoFlash, Lldb, AesCapsule, KvDriver, PattgenCapsule, SystemResetControllerCapsule,
    AlertHandlerCapsule,
};
use crate::{Chip, DefaultPeripherals, Platform, Scheduler};

/// The context provided for Tock's `main` file.
///
/// This should be created from a [`Configuration`], as it's meant to be the glue between
/// the user's agnostic configuration and the Tock's specific internals needed for the code generation
/// process.
pub struct Context<C: Chip> {
    pub platform: Rc<Platform<C>>,
    pub chip: Rc<C>,
    pub process_count: usize,
    pub stack_size: usize,
}

impl<C: Chip> Context<C> {
    pub fn from_config(
        chip: C,
        config: Configuration<C::Peripherals>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut visited = Vec::new();
        let mut capsules = Vec::new();

        // Iterate over the capsules and insert them into the current platform's capsule list.
        for capsule_config in config.capsules() {
            match capsule_config {
                Capsule::Console { uart, baud_rate } => {
                    let mux_uart = MuxUart::insert_get(Rc::clone(uart), *baud_rate, &mut visited);
                    capsules.push(Console::get(mux_uart) as Rc<dyn crate::Capsule>)
                }
                Capsule::Alarm { timer } => {
                    let mux_alarm = MuxAlarm::insert_get(Rc::clone(timer), &mut visited);
                    capsules.push(AlarmDriver::get(mux_alarm) as Rc<dyn crate::Capsule>)
                }
                Capsule::Temperature { temp } => capsules
                    .push(TemperatureCapsule::get(Rc::clone(temp)) as Rc<dyn crate::Capsule>),
                Capsule::Rng { rng } => {
                    capsules.push(RngCapsule::get(Rc::clone(rng)) as Rc<dyn crate::Capsule>)
                }
                Capsule::Spi { spi } =>
                    capsules.push(SpiCapsule::get(Rc::clone(spi)) as Rc<dyn crate::Capsule>),
                Capsule::I2c { i2c } =>
                    capsules.push(I2CMasterDriver::get(Rc::clone(i2c)) as Rc<dyn crate::Capsule>),
                Capsule::Gpio { pins } =>
                    capsules.push(GPIO::<<<C as Chip>::Peripherals as DefaultPeripherals>::Gpio>::get(pins.clone()) as Rc<dyn crate::Capsule>),
                Capsule::Led { led_type, pins } =>
                    capsules.push(Led::<<<C as Chip>::Peripherals as DefaultPeripherals>::Gpio>::get(*led_type, pins.clone()) as Rc<dyn crate::Capsule>),
                Capsule::Hmac { hmac, length } =>
                    capsules.push(HmacCapsule::get(Rc::clone(hmac), *length) as Rc<dyn crate::Capsule>),
                Capsule::KvDriver { flash } => {
                    capsules.push(KvDriver::get(flash.clone()) as Rc<dyn crate::Capsule>);
                }
                Capsule::InfoFlash { flash } =>
                    capsules.push(InfoFlash::get(Rc::clone(flash)) as Rc<dyn crate::Capsule>),
                Capsule::Lldb { uart, baud_rate } => {
                    let mux_uart = MuxUart::insert_get(Rc::clone(uart), *baud_rate, &mut visited);
                    capsules.push(Lldb::get(mux_uart) as Rc<dyn crate::Capsule>);
                }
                Capsule::Aes { aes, number_of_blocks } => {
                    capsules.push(AesCapsule::get(aes.clone(), *number_of_blocks) as Rc<dyn crate::Capsule>);
                }
                Capsule::Pattgen { pattgen } => {
                    capsules.push(PattgenCapsule::get(pattgen.clone()) as Rc<dyn crate::Capsule>);
                }
                Capsule::SystemResetController { system_reset_controller } => {
                    capsules.push(SystemResetControllerCapsule::get(system_reset_controller.clone()) as Rc<dyn crate::Capsule>);
                }
                Capsule::AlertHandler { alert_handler } => {
                    capsules.push(AlertHandlerCapsule::get(alert_handler.clone()) as Rc<dyn crate::Capsule>);
                }
                _ => {}
            };
        }

        Ok(Self {
            platform: Rc::new(Platform::<C>::new(
                config.r#type,
                capsules,
                Scheduler::insert_get(config.scheduler, &mut visited),
                chip.systick()?,
            )),
            chip: Rc::new(chip),
            process_count: config.process_count,
            stack_size: config.stack_size.into(),
        })
    }
}
