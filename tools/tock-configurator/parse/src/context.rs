// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use std::error::Error;
use std::rc::Rc;

use crate::config::{Capsule, Configuration};
use crate::Component;
use crate::{
    AesCapsule, AlarmDriver, AlertHandlerCapsule, AsymmetricCryptoCapsule, AttestationCapsule,
    Console, HmacCapsule, I2CMasterDriver, InfoFlash, KvDriver, Led, Lldb, MuxAlarm, MuxUart,
    OneshotCshake128Capsule, OneshotCshake256Capsule, OneshotHmacSha256Capsule,
    OneshotHmacSha384Capsule, OneshotHmacSha512Capsule, OneshotKmac128Capsule,
    OneshotKmac256Capsule, OneshotSha256Capsule, OneshotSha384Capsule, OneshotSha3_224Capsule,
    OneshotSha3_256Capsule, OneshotSha3_384Capsule, OneshotSha3_512Capsule, OneshotSha512Capsule,
    OneshotShake128Capsule, OneshotShake256Capsule, PattgenCapsule, ResetManagerCapsule,
    RngCapsule, SpiCapsule, SystemResetControllerCapsule, TemperatureCapsule, UsbCapsule, GPIO,
    IPC,
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
        config: Configuration<<C as Chip>::Peripherals>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut visited = Vec::new();
        let mut capsules = Vec::new();
        let temp = chip.peripheral_config();
        let mut peripheral_config = temp.borrow_mut();

        // Iterate over the capsules and insert them into the current platform's
        // capsule list.
        //
        // Also, run the tracer to determine which drivers and virtualizers to
        // include and interrupts to enable.
        for capsule_config in config.capsules() {
            match capsule_config {
                Capsule::Console { uart, baud_rate } => {
                    let mux_uart = MuxUart::insert_get(Rc::clone(uart), *baud_rate, &mut visited);
                    let console = Console::get(mux_uart);
                    console.trace_dependencies(&mut *peripheral_config);
                    capsules.push(console as Rc<dyn crate::Capsule>)
                }
                Capsule::Alarm { timer } => {
                    let mux_alarm = MuxAlarm::insert_get(Rc::clone(timer), &mut visited);
                    let alarm_driver = AlarmDriver::get(mux_alarm);
                    alarm_driver.trace_dependencies(&mut *peripheral_config);
                    capsules.push(alarm_driver as Rc<dyn crate::Capsule>)
                }
                Capsule::Temperature { temp } => {
                    let temperature = TemperatureCapsule::get(Rc::clone(temp));
                    temperature.trace_dependencies(&mut *peripheral_config);
                    capsules.push(temperature as Rc<dyn crate::Capsule>)
                }
                Capsule::Rng { rng } => {
                    let rng = RngCapsule::get(Rc::clone(rng));
                    rng.trace_dependencies(&mut *peripheral_config);
                    capsules.push(rng as Rc<dyn crate::Capsule>)
                }
                Capsule::Spi { spi } => {
                    let spi = SpiCapsule::get(Rc::clone(spi));
                    spi.trace_dependencies(&mut *peripheral_config);
                    capsules.push(spi as Rc<dyn crate::Capsule>)
                }
                Capsule::I2c { i2c } => {
                    let i2c = I2CMasterDriver::get(Rc::clone(i2c));
                    i2c.trace_dependencies(&mut *peripheral_config);
                    capsules.push(i2c as Rc<dyn crate::Capsule>)
                }
                Capsule::Gpio { pins } => {
                    let gpio = GPIO::<<<C as Chip>::Peripherals as DefaultPeripherals>::Gpio>::get(
                        pins.clone(),
                    );
                    gpio.trace_dependencies(&mut *peripheral_config);
                    capsules.push(gpio as Rc<dyn crate::Capsule>)
                }
                Capsule::Led { led_type, pins } => {
                    let led = Led::<<<C as Chip>::Peripherals as DefaultPeripherals>::Gpio>::get(
                        *led_type,
                        pins.clone(),
                    );
                    led.trace_dependencies(&mut *peripheral_config);
                    capsules.push(led as Rc<dyn crate::Capsule>)
                }
                Capsule::Hmac { hmac, length } => {
                    let hmac = HmacCapsule::get(Rc::clone(hmac), *length);
                    hmac.trace_dependencies(&mut *peripheral_config);
                    capsules.push(hmac as Rc<dyn crate::Capsule>)
                }
                Capsule::KvDriver { flash } => {
                    let kv_driver = KvDriver::get(Rc::clone(flash));
                    kv_driver.trace_dependencies(&mut *peripheral_config);
                    capsules.push(kv_driver as Rc<dyn crate::Capsule>);
                }
                Capsule::InfoFlash { flash } => {
                    let info_flash = InfoFlash::get(Rc::clone(&flash));
                    info_flash.trace_dependencies(&mut *peripheral_config);
                    capsules.push(info_flash as Rc<dyn crate::Capsule>)
                }
                Capsule::Lldb { uart, baud_rate } => {
                    let mux_uart = MuxUart::insert_get(Rc::clone(uart), *baud_rate, &mut visited);
                    let lldb = Lldb::get(mux_uart);
                    lldb.trace_dependencies(&mut *peripheral_config);
                    capsules.push(lldb as Rc<dyn crate::Capsule>);
                }
                Capsule::Aes {
                    aes,
                    number_of_blocks,
                } => {
                    let aes = AesCapsule::get(aes.clone(), *number_of_blocks);
                    aes.trace_dependencies(&mut *peripheral_config);
                    capsules.push(aes as Rc<dyn crate::Capsule>);
                }
                Capsule::Pattgen { pattgen } => {
                    let pattgen = PattgenCapsule::get(pattgen.clone());
                    pattgen.trace_dependencies(&mut *peripheral_config);
                    capsules.push(pattgen as Rc<dyn crate::Capsule>);
                }
                Capsule::SystemResetController {
                    system_reset_controller,
                } => {
                    let sysreset =
                        SystemResetControllerCapsule::get(system_reset_controller.clone());
                    sysreset.trace_dependencies(&mut *peripheral_config);
                    capsules.push(sysreset as Rc<dyn crate::Capsule>);
                }
                Capsule::AlertHandler { alert_handler } => {
                    let alert_handler = AlertHandlerCapsule::get(alert_handler.clone());
                    alert_handler.trace_dependencies(&mut *peripheral_config);
                    capsules.push(alert_handler as Rc<dyn crate::Capsule>);
                }
                Capsule::Usb { usb } => {
                    let usb = UsbCapsule::get(usb.clone());
                    usb.trace_dependencies(&mut *peripheral_config);
                    capsules.push(usb as Rc<dyn crate::Capsule>);
                }
                Capsule::ResetManager { reset_manager } => {
                    let reset_manager = ResetManagerCapsule::get(reset_manager.clone());
                    reset_manager.trace_dependencies(&mut *peripheral_config);
                    capsules.push(reset_manager as Rc<dyn crate::Capsule>);
                }
                Capsule::Ipc {} => {
                    let ipc = IPC::get();
                    ipc.trace_dependencies(&mut *peripheral_config);
                    capsules.push(ipc as Rc<dyn crate::Capsule>);
                }
                Capsule::Attestation { attestation } => {
                    let attestation = AttestationCapsule::get(Rc::clone(attestation));
                    attestation.trace_dependencies(&mut *peripheral_config);
                    capsules.push(attestation as Rc<dyn crate::Capsule>);
                }
                Capsule::P256 { p256 } => {
                    let p256 = AsymmetricCryptoCapsule::get(
                        "DRIVER_NUM_P256".to_string(),
                        "<kernel::hil::public_key_crypto::ecc::P256 as kernel::hil::public_key_crypto::ecc::EllipticCurve>::HASH_LEN".to_string(),
                        "<kernel::hil::public_key_crypto::ecc::P256 as kernel::hil::public_key_crypto::ecc::EllipticCurve>::SIG_LEN".to_string(),
                        "EcdsaP256".to_string(),
                        p256.clone(),
                    );
                    p256.trace_dependencies(&mut *peripheral_config);
                    capsules.push(p256 as Rc<dyn crate::Capsule>);
                }
                Capsule::P384 { p384 } => {
                    let p384 = AsymmetricCryptoCapsule::get(
                        "DRIVER_NUM_P384".to_string(),
                        "<kernel::hil::public_key_crypto::ecc::P384 as kernel::hil::public_key_crypto::ecc::EllipticCurve>::HASH_LEN".to_string(),
                        "<kernel::hil::public_key_crypto::ecc::P384 as kernel::hil::public_key_crypto::ecc::EllipticCurve>::SIG_LEN".to_string(),
                        "EcdsaP384".to_string(),
                        p384.clone(),
                    );
                    p384.trace_dependencies(&mut *peripheral_config);
                    capsules.push(p384 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotSha256 { oneshot_sha256 } => {
                    let oneshot_sha256 = OneshotSha256Capsule::get(oneshot_sha256.clone());
                    oneshot_sha256.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_sha256 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotSha384 { oneshot_sha384 } => {
                    let oneshot_sha384 = OneshotSha384Capsule::get(oneshot_sha384.clone());
                    oneshot_sha384.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_sha384 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotSha512 { oneshot_sha512 } => {
                    let oneshot_sha512 = OneshotSha512Capsule::get(oneshot_sha512.clone());
                    oneshot_sha512.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_sha512 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotSha3_224 { oneshot_sha3_224 } => {
                    let oneshot_sha3_224 = OneshotSha3_224Capsule::get(oneshot_sha3_224.clone());
                    oneshot_sha3_224.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_sha3_224 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotSha3_256 { oneshot_sha3_256 } => {
                    let oneshot_sha3_256 = OneshotSha3_256Capsule::get(oneshot_sha3_256.clone());
                    oneshot_sha3_256.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_sha3_256 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotSha3_384 { oneshot_sha3_384 } => {
                    let oneshot_sha3_384 = OneshotSha3_384Capsule::get(oneshot_sha3_384.clone());
                    oneshot_sha3_384.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_sha3_384 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotSha3_512 { oneshot_sha3_512 } => {
                    let oneshot_sha3_512 = OneshotSha3_512Capsule::get(oneshot_sha3_512.clone());
                    oneshot_sha3_512.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_sha3_512 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotShake128 { oneshot_shake128 } => {
                    let oneshot_shake128 = OneshotShake128Capsule::get(oneshot_shake128.clone());

                    oneshot_shake128.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_shake128 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotShake256 { oneshot_shake256 } => {
                    let oneshot_shake256 = OneshotShake256Capsule::get(oneshot_shake256.clone());

                    oneshot_shake256.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_shake256 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotCshake128 { oneshot_cshake128 } => {
                    let oneshot_cshake128 = OneshotCshake128Capsule::get(oneshot_cshake128.clone());

                    oneshot_cshake128.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_cshake128 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotCshake256 { oneshot_cshake256 } => {
                    let oneshot_cshake256 = OneshotCshake256Capsule::get(oneshot_cshake256.clone());

                    oneshot_cshake256.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_cshake256 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotHmacSha256 {
                    oneshot_hmac_sha256,
                } => {
                    let oneshot_hmac_sha256 =
                        OneshotHmacSha256Capsule::get(oneshot_hmac_sha256.clone());
                    oneshot_hmac_sha256.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_hmac_sha256 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotHmacSha384 {
                    oneshot_hmac_sha384,
                } => {
                    let oneshot_hmac_sha384 =
                        OneshotHmacSha384Capsule::get(oneshot_hmac_sha384.clone());
                    oneshot_hmac_sha384.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_hmac_sha384 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotHmacSha512 {
                    oneshot_hmac_sha512,
                } => {
                    let oneshot_hmac_sha512 =
                        OneshotHmacSha512Capsule::get(oneshot_hmac_sha512.clone());
                    oneshot_hmac_sha512.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_hmac_sha512 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotKmac128 { oneshot_kmac128 } => {
                    let oneshot_kmac128 = OneshotKmac128Capsule::get(oneshot_kmac128.clone());

                    oneshot_kmac128.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_kmac128 as Rc<dyn crate::Capsule>);
                }
                Capsule::OneshotKmac256 { oneshot_kmac256 } => {
                    let oneshot_kmac256 = OneshotKmac256Capsule::get(oneshot_kmac256.clone());

                    oneshot_kmac256.trace_dependencies(&mut *peripheral_config);
                    capsules.push(oneshot_kmac256 as Rc<dyn crate::Capsule>);
                }
                _ => unreachable!("Capsule context branch not set."),
            };
        }
        let scheduler = Scheduler::insert_get(config.scheduler, &mut visited);
        let systick = chip.systick()?;
        let watchdog = chip.watchdog()?;

        scheduler.trace_dependencies(&mut *peripheral_config);
        systick.trace_dependencies(&mut *peripheral_config);
        watchdog.trace_dependencies(&mut *peripheral_config);

        Ok(Self {
            platform: Rc::new(Platform::<C>::new(
                config.r#type,
                capsules,
                scheduler,
                systick,
                watchdog,
            )),
            chip: Rc::new(chip),
            process_count: config.process_count,
            stack_size: config.stack_size.into(),
        })
    }
}
