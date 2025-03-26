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
    AesCapsule, AlarmDriver, AlertHandlerCapsule, AsymmetricCryptoCapsule, AttestationCapsule,
    Console, HmacCapsule, I2CMasterDriver, InfoFlash, KvDriver, Led, Lldb, MuxAlarm, MuxUart,
    OneshotDigestCapsule, PattgenCapsule, ResetManagerCapsule, RngCapsule, SpiCapsule,
    SystemResetControllerCapsule, TemperatureCapsule, UsbCapsule, GPIO, IPC,
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
                Capsule::Spi { spi } => {
                    capsules.push(SpiCapsule::get(Rc::clone(spi)) as Rc<dyn crate::Capsule>)
                }
                Capsule::I2c { i2c } => {
                    capsules.push(I2CMasterDriver::get(Rc::clone(i2c)) as Rc<dyn crate::Capsule>)
                }
                Capsule::Gpio { pins } => capsules.push(GPIO::<
                    <<C as Chip>::Peripherals as DefaultPeripherals>::Gpio,
                >::get(pins.clone())
                    as Rc<dyn crate::Capsule>),
                Capsule::Led { led_type, pins } => capsules.push(Led::<
                    <<C as Chip>::Peripherals as DefaultPeripherals>::Gpio,
                >::get(
                    *led_type, pins.clone()
                )
                    as Rc<dyn crate::Capsule>),
                Capsule::Hmac { hmac, length } => {
                    capsules
                        .push(HmacCapsule::get(Rc::clone(hmac), *length) as Rc<dyn crate::Capsule>)
                }
                Capsule::KvDriver { flash } => {
                    capsules.push(KvDriver::get(flash.clone()) as Rc<dyn crate::Capsule>);
                }
                Capsule::InfoFlash { flash } => {
                    capsules.push(InfoFlash::get(Rc::clone(flash)) as Rc<dyn crate::Capsule>)
                }
                Capsule::Lldb { uart, baud_rate } => {
                    let mux_uart = MuxUart::insert_get(Rc::clone(uart), *baud_rate, &mut visited);
                    capsules.push(Lldb::get(mux_uart) as Rc<dyn crate::Capsule>);
                }
                Capsule::Aes {
                    aes,
                    number_of_blocks,
                } => {
                    capsules
                        .push(AesCapsule::get(aes.clone(), *number_of_blocks)
                            as Rc<dyn crate::Capsule>);
                }
                Capsule::Pattgen { pattgen } => {
                    capsules.push(PattgenCapsule::get(pattgen.clone()) as Rc<dyn crate::Capsule>);
                }
                Capsule::SystemResetController {
                    system_reset_controller,
                } => {
                    capsules.push(
                        SystemResetControllerCapsule::get(system_reset_controller.clone())
                            as Rc<dyn crate::Capsule>,
                    );
                }
                Capsule::AlertHandler { alert_handler } => {
                    capsules
                        .push(AlertHandlerCapsule::get(alert_handler.clone())
                            as Rc<dyn crate::Capsule>);
                }
                Capsule::Usb { usb } => {
                    capsules.push(UsbCapsule::get(usb.clone()) as Rc<dyn crate::Capsule>);
                }
                Capsule::ResetManager { reset_manager } => {
                    capsules
                        .push(ResetManagerCapsule::get(reset_manager.clone())
                            as Rc<dyn crate::Capsule>);
                }
                Capsule::Ipc {} => {
                    capsules.push(IPC::get() as Rc<dyn crate::Capsule>);
                }
                Capsule::Attestation { attestation } => {
                    capsules.push(
                        AttestationCapsule::get(attestation.clone()) as Rc<dyn crate::Capsule>
                    );
                }
                Capsule::P256 { p256 } => {
                    capsules.push(AsymmetricCryptoCapsule::get(
                        "DRIVER_NUM_P256".to_string(),
                        "<kernel::hil::public_key_crypto::ecc::P256 as kernel::hil::public_key_crypto::ecc::EllipticCurve>::HASH_LEN".to_string(),
                        "<kernel::hil::public_key_crypto::ecc::P256 as kernel::hil::public_key_crypto::ecc::EllipticCurve>::SIG_LEN".to_string(),
                        "EcdsaP256".to_string(),
                        p256.clone(),
                    ) as Rc<dyn crate::Capsule>);
                }
                Capsule::P384 { p384 } => {
                    capsules.push(AsymmetricCryptoCapsule::get(
                        "DRIVER_NUM_P384".to_string(),
                        "<kernel::hil::public_key_crypto::ecc::P384 as kernel::hil::public_key_crypto::ecc::EllipticCurve>::HASH_LEN".to_string(),
                        "<kernel::hil::public_key_crypto::ecc::P384 as kernel::hil::public_key_crypto::ecc::EllipticCurve>::SIG_LEN".to_string(),
                        "EcdsaP384".to_string(),
                        p384.clone(),
                    ) as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotDigest { oneshot_digest } => {
                    capsules
                        .push(OneshotDigestCapsule::get(oneshot_digest.clone())
                            as Rc<dyn crate::Capsule>);
                }
                _ => unreachable!("Capsule context branch not set."),
            };
        }

        Ok(Self {
            platform: Rc::new(Platform::<C>::new(
                config.r#type,
                capsules,
                Scheduler::insert_get(config.scheduler, &mut visited),
                chip.systick()?,
                chip.watchdog()?,
            )),
            chip: Rc::new(chip),
            process_count: config.process_count,
            stack_size: config.stack_size.into(),
        })
    }
}
