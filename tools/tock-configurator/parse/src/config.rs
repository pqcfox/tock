// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use parse_macros::capsules_config;

use crate::LedType;
use crate::{DefaultPeripherals, SchedulerType, SyscallFilterType};
use crate::{Lsm303AccelDataRate, Lsm303MagnetoDataRate, Lsm303Range, Lsm303Scale};
use std::{collections::HashMap, num::NonZeroUsize, rc::Rc};
pub type CapsulesConfigurations<P> = HashMap<Index, Capsule<P>>;

capsules_config!(
    Index => Capsule<P: crate::DefaultPeripherals>,
    // The keys and values enums for the capsules map.
    {
        LLDB => Lldb { uart: Rc<P::Uart>, baud_rate: usize },
        CONSOLE => Console { uart: Rc<P::Uart>, baud_rate: usize},
        ALARM => Alarm { timer: Rc<P::Timer> },
        LED => Led { led_type: LedType, pins: Vec<<P::Gpio as crate::Gpio>::PinId> },
        SPI => Spi { spi: Rc<P::Spi> },
        I2C => I2c { i2c: Rc<P::I2c> },
        BLE => BleRadio { ble: Rc<P::BleAdvertisement>, timer: Rc<P::Timer> },
        FLASH => Flash { flash: Rc<P::Flash>, buffer_size: usize },
        LSM303AGR => Lsm303agr { i2c: Rc<P::I2c>,
                                 accel_data_rate: Lsm303AccelDataRate,
                                 low_power: bool,
                                 accel_scale: Lsm303Scale,
                                 accel_high_resolution: bool,
                                 temperature: bool,
                                 mag_data_rate: Lsm303MagnetoDataRate,
                                 mag_range: Lsm303Range  },

        TEMPERATURE => Temperature { temp: Rc<P::Temperature> },
        RNG => Rng { rng: Rc<P::Rng> },
        GPIO => Gpio { pins: Vec<<P::Gpio as crate::Gpio>::PinId> },
        HMAC => Hmac { hmac: Rc<P::Hmac>, length: usize },
        KV_DRIVER => KvDriver { flash: Rc<P::Flash> },
        INFO_FLASH => InfoFlash { flash: Rc<P::Flash> },
        AES => Aes { aes: Rc<P::Aes>, number_of_blocks: usize },
        PATTGEN => Pattgen { pattgen: Rc<P::Pattgen> },
        SYSTEM_RESET_CONTROLLER => SystemResetController { system_reset_controller: Rc<P::SystemResetController> },
        ALERT_HANDLER => AlertHandler { alert_handler: Rc<P::AlertHandler> },
        USB => Usb { usb: Rc<P::Usb> },
        RESET_MANAGER => ResetManager { reset_manager: Rc<P::ResetManager> },
        IPC => Ipc {},
        ATTESTATION => Attestation { attestation: Rc<P::Attestation> },
        ONESHOT_SHA256 => OneshotSha256 { oneshot_sha256: Rc<P::OneshotSha256> },
        ONESHOT_SHA384 => OneshotSha384 { oneshot_sha384: Rc<P::OneshotSha384> },
        ONESHOT_SHA512 => OneshotSha512 { oneshot_sha512: Rc<P::OneshotSha512> },
        ONESHOT_SHA3_224 => OneshotSha3_224 { oneshot_sha3_224: Rc<P::OneshotSha3_224> },
        ONESHOT_SHA3_256 => OneshotSha3_256 { oneshot_sha3_256: Rc<P::OneshotSha3_256> },
        ONESHOT_SHA3_384 => OneshotSha3_384 { oneshot_sha3_384: Rc<P::OneshotSha3_384> },
        ONESHOT_SHA3_512 => OneshotSha3_512 { oneshot_sha3_512: Rc<P::OneshotSha3_512> },
        ONESHOT_SHAKE128 => OneshotShake128 { oneshot_shake128: Rc<P::OneshotShake128> },
        ONESHOT_SHAKE256 => OneshotShake256 { oneshot_shake256: Rc<P::OneshotShake256> },
        ONESHOT_CSHAKE128 => OneshotCshake128 { oneshot_cshake128: Rc<P::OneshotCshake128> },
        ONESHOT_CSHAKE256 => OneshotCshake256 { oneshot_cshake256: Rc<P::OneshotCshake256> },
        ONESHOT_HMAC_SHA256 => OneshotHmacSha256 { oneshot_hmac_sha256: Rc<P::OneshotHmacSha256> },
        ONESHOT_HMAC_SHA384 => OneshotHmacSha384 { oneshot_hmac_sha384: Rc<P::OneshotHmacSha384> },
        ONESHOT_HMAC_SHA512 => OneshotHmacSha512 { oneshot_hmac_sha512: Rc<P::OneshotHmacSha512> },
        ONESHOT_KMAC128 => OneshotKmac128 { oneshot_kmac128: Rc<P::OneshotKmac128> },
        ONESHOT_KMAC256 => OneshotKmac256 { oneshot_kmac256: Rc<P::OneshotKmac256> },
        P256 => P256 { p256: Rc<P::P256> },
        P384 => P384 { p384: Rc<P::P384> },
    }
);

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Configuration<P: DefaultPeripherals> {
    // The type of the board struct configured.
    // Considered neither optional nor required,
    // but more of a way to integrate with already-defined Tock platforms.
    pub r#type: String,

    // Capsules are the optional configuration fields.
    // The map representation removes the redundancy of having
    // multiple optional fields for serialization purposes.
    capsules: CapsulesConfigurations<P>,

    // The required configuration fields for the platform.
    pub scheduler: SchedulerType,
    pub process_count: usize,
    pub stack_size: NonZeroUsize,
    pub syscall_filter: SyscallFilterType,
}

impl<P: DefaultPeripherals> Default for Configuration<P> {
    fn default() -> Self {
        Self {
            r#type: String::from("AutogeneratedPlatform"),
            capsules: Default::default(),
            scheduler: Default::default(),
            process_count: Default::default(),
            stack_size: unsafe { NonZeroUsize::new_unchecked(0x900) },
            syscall_filter: SyscallFilterType::None,
        }
    }
}

macro_rules! capsule_single_param {
    {
        update = $update:ident,
        remove = $remove:ident,
        capsule_uc = $capsule_uc:ident,
        capsule_sc = $capsule_sc:ident,
        peripheral_sc = $peripheral_sc:ident,
        peripheral_lc = $peripheral_lc:ident,
    } => {
        #[doc = concat!(
            "Update the ",
            stringify!($capsule_sc),
            " configuration.",
        )]
        pub fn $update(&mut self, $peripheral_lc: Rc<P::$peripheral_sc>) {
            // Remove the other entry and decrement its dependent count, in case
            // this `update` uses a different peripheral.
            self.capsules
                .insert(Index::$capsule_uc, Capsule::$capsule_sc { $peripheral_lc });
        }

        #[doc = concat!(
            "Remove the ",
            stringify!($capsule_sc),
            " configuration.",
        )]
        pub fn $remove(&mut self) {
            self.capsules.remove(&Index::$capsule_uc);
        }
    }
}

// Configuration methods exposed for the `configurator` crate.
impl<P: DefaultPeripherals> Configuration<P> {
    /// Return a vector of the configured capsules.
    pub fn capsules(&self) -> Vec<&Capsule<P>> {
        self.capsules.values().collect()
    }

    pub fn capsule(&self, capsule: &Index) -> Option<&Capsule<P>> {
        self.capsules.get(capsule)
    }

    capsule_single_param! {
        update = update_alarm,
        remove = remove_alarm,
        capsule_uc = ALARM,
        capsule_sc = Alarm,
        peripheral_sc = Timer,
        peripheral_lc = timer,
    }
    capsule_single_param! {
        update = update_spi,
        remove = remove_spi,
        capsule_uc = SPI,
        capsule_sc = Spi,
        peripheral_sc = Spi,
        peripheral_lc = spi,
    }
    capsule_single_param! {
        update = update_i2c,
        remove = remove_i2c,
        capsule_uc = I2C,
        capsule_sc = I2c,
        peripheral_sc = I2c,
        peripheral_lc = i2c,
    }
    capsule_single_param! {
        update = update_temp,
        remove = remove_temp,
        capsule_uc = TEMPERATURE,
        capsule_sc = Temperature,
        peripheral_sc = Temperature,
        peripheral_lc = temp,
    }
    capsule_single_param! {
        update = update_rng,
        remove = remove_rng,
        capsule_uc = RNG,
        capsule_sc = Rng,
        peripheral_sc = Rng,
        peripheral_lc = rng,
    }
    capsule_single_param! {
        update = update_kv_driver,
        remove = remove_kv_driver,
        capsule_uc = KV_DRIVER,
        capsule_sc = KvDriver,
        peripheral_sc = Flash,
        peripheral_lc = flash,
    }
    capsule_single_param! {
        update = update_pattgen,
        remove = remove_pattgen,
        capsule_uc = PATTGEN,
        capsule_sc = Pattgen,
        peripheral_sc = Pattgen,
        peripheral_lc = pattgen,
    }
    capsule_single_param! {
        update = update_system_reset_controller,
        remove = remove_system_reset_controller,
        capsule_uc = SYSTEM_RESET_CONTROLLER,
        capsule_sc = SystemResetController,
        peripheral_sc = SystemResetController,
        peripheral_lc = system_reset_controller,
    }
    capsule_single_param! {
        update = update_alert_handler,
        remove = remove_alert_handler,
        capsule_uc = ALERT_HANDLER,
        capsule_sc = AlertHandler,
        peripheral_sc = AlertHandler,
        peripheral_lc = alert_handler,
    }
    capsule_single_param! {
        update = update_usb,
        remove = remove_usb,
        capsule_uc = USB,
        capsule_sc = Usb,
        peripheral_sc = Usb,
        peripheral_lc = usb,
    }
    capsule_single_param! {
        update = update_reset_manager,
        remove = remove_reset_manager,
        capsule_uc = RESET_MANAGER,
        capsule_sc = ResetManager,
        peripheral_sc = ResetManager,
        peripheral_lc = reset_manager,
    }
    capsule_single_param! {
        update = update_attestation,
        remove = remove_attestation,
        capsule_uc = ATTESTATION,
        capsule_sc = Attestation,
        peripheral_sc = Attestation,
        peripheral_lc = attestation,
    }
    capsule_single_param! {
        update = update_oneshot_sha256,
        remove = remove_oneshot_sha256,
        capsule_uc = ONESHOT_SHA256,
        capsule_sc = OneshotSha256,
        peripheral_sc = OneshotSha256,
        peripheral_lc = oneshot_sha256,
    }
    capsule_single_param! {
        update = update_oneshot_sha384,
        remove = remove_oneshot_sha384,
        capsule_uc = ONESHOT_SHA384,
        capsule_sc = OneshotSha384,
        peripheral_sc = OneshotSha384,
        peripheral_lc = oneshot_sha384,
    }
    capsule_single_param! {
        update = update_oneshot_sha512,
        remove = remove_oneshot_sha512,
        capsule_uc = ONESHOT_SHA512,
        capsule_sc = OneshotSha512,
        peripheral_sc = OneshotSha512,
        peripheral_lc = oneshot_sha512,
    }
    capsule_single_param! {
        update = update_oneshot_sha3_224,
        remove = remove_oneshot_sha3_224,
        capsule_uc = ONESHOT_SHA3_224,
        capsule_sc = OneshotSha3_224,
        peripheral_sc = OneshotSha3_224,
        peripheral_lc = oneshot_sha3_224,
    }
    capsule_single_param! {
        update = update_oneshot_sha3_256,
        remove = remove_oneshot_sha3_256,
        capsule_uc = ONESHOT_SHA3_256,
        capsule_sc = OneshotSha3_256,
        peripheral_sc = OneshotSha3_256,
        peripheral_lc = oneshot_sha3_256,
    }
    capsule_single_param! {
        update = update_oneshot_sha3_384,
        remove = remove_oneshot_sha3_384,
        capsule_uc = ONESHOT_SHA3_384,
        capsule_sc = OneshotSha3_384,
        peripheral_sc = OneshotSha3_384,
        peripheral_lc = oneshot_sha3_384,
    }
    capsule_single_param! {
        update = update_oneshot_sha3_512,
        remove = remove_oneshot_sha3_512,
        capsule_uc = ONESHOT_SHA3_512,
        capsule_sc = OneshotSha3_512,
        peripheral_sc = OneshotSha3_512,
        peripheral_lc = oneshot_sha3_512,
    }
    capsule_single_param! {
        update = update_oneshot_shake128,
        remove = remove_oneshot_shake128,
        capsule_uc = ONESHOT_SHAKE128,
        capsule_sc = OneshotShake128,
        peripheral_sc = OneshotShake128,
        peripheral_lc = oneshot_shake128,
    }
    capsule_single_param! {
        update = update_oneshot_shake256,
        remove = remove_oneshot_shake256,
        capsule_uc = ONESHOT_SHAKE256,
        capsule_sc = OneshotShake256,
        peripheral_sc = OneshotShake256,
        peripheral_lc = oneshot_shake256,
    }
    capsule_single_param! {
        update = update_oneshot_cshake128,
        remove = remove_oneshot_cshake128,
        capsule_uc = ONESHOT_CSHAKE128,
        capsule_sc = OneshotCshake128,
        peripheral_sc = OneshotCshake128,
        peripheral_lc = oneshot_cshake128,
    }
    capsule_single_param! {
        update = update_oneshot_cshake256,
        remove = remove_oneshot_cshake256,
        capsule_uc = ONESHOT_CSHAKE256,
        capsule_sc = OneshotCshake256,
        peripheral_sc = OneshotCshake256,
        peripheral_lc = oneshot_cshake256,
    }
    capsule_single_param! {
        update = update_oneshot_hmac_sha256,
        remove = remove_oneshot_hmac_sha256,
        capsule_uc = ONESHOT_HMAC_SHA256,
        capsule_sc = OneshotHmacSha256,
        peripheral_sc = OneshotHmacSha256,
        peripheral_lc = oneshot_hmac_sha256,
    }
    capsule_single_param! {
        update = update_oneshot_hmac_sha384,
        remove = remove_oneshot_hmac_sha384,
        capsule_uc = ONESHOT_HMAC_SHA384,
        capsule_sc = OneshotHmacSha384,
        peripheral_sc = OneshotHmacSha384,
        peripheral_lc = oneshot_hmac_sha384,
    }
    capsule_single_param! {
        update = update_oneshot_hmac_sha512,
        remove = remove_oneshot_hmac_sha512,
        capsule_uc = ONESHOT_HMAC_SHA512,
        capsule_sc = OneshotHmacSha512,
        peripheral_sc = OneshotHmacSha512,
        peripheral_lc = oneshot_hmac_sha512,
    }
    capsule_single_param! {
        update = update_oneshot_kmac128,
        remove = remove_oneshot_kmac128,
        capsule_uc = ONESHOT_KMAC128,
        capsule_sc = OneshotKmac128,
        peripheral_sc = OneshotKmac128,
        peripheral_lc = oneshot_kmac128,
    }
    capsule_single_param! {
        update = update_oneshot_kmac256,
        remove = remove_oneshot_kmac256,
        capsule_uc = ONESHOT_KMAC256,
        capsule_sc = OneshotKmac256,
        peripheral_sc = OneshotKmac256,
        peripheral_lc = oneshot_kmac256,
    }
    capsule_single_param! {
        update = update_p256,
        remove = remove_p256,
        capsule_uc = P256,
        capsule_sc = P256,
        peripheral_sc = P256,
        peripheral_lc = p256,
    }
    capsule_single_param! {
        update = update_p384,
        remove = remove_p384,
        capsule_uc = P384,
        capsule_sc = P384,
        peripheral_sc = P384,
        peripheral_lc = p384,
    }

    /// Update the console configuration.
    pub fn update_console(&mut self, uart: Rc<P::Uart>, baud_rate: usize) {
        self.capsules
            .insert(Index::CONSOLE, Capsule::Console { uart, baud_rate });
    }

    /// Update the ble configuration.
    pub fn update_ble(&mut self, ble: Rc<P::BleAdvertisement>, timer: Rc<P::Timer>) {
        self.capsules
            .insert(Index::BLE, Capsule::BleRadio { ble, timer });
    }

    /// Update the lsm303agr configuration.
    // FIXME: Move the LSM config to a struct.
    #[allow(clippy::too_many_arguments)]
    pub fn update_lsm303agr(
        &mut self,
        i2c: Rc<P::I2c>,
        accel_data_rate: Lsm303AccelDataRate,
        low_power: bool,
        accel_scale: Lsm303Scale,
        accel_high_resolution: bool,
        temperature: bool,
        mag_data_rate: Lsm303MagnetoDataRate,
        mag_range: Lsm303Range,
    ) {
        #[allow(clippy::too_many_arguments)]
        self.capsules.insert(
            Index::LSM303AGR,
            Capsule::Lsm303agr {
                i2c,
                accel_data_rate,
                low_power,
                accel_scale,
                accel_high_resolution,
                temperature,
                mag_data_rate,
                mag_range,
            },
        );
    }

    /// Update the flash configuration.
    pub fn update_flash(&mut self, flash: Rc<P::Flash>, buffer_size: usize) {
        self.capsules
            .insert(Index::FLASH, Capsule::Flash { flash, buffer_size });
    }

    /// Update the flash configuration.
    pub fn update_info_flash(&mut self, flash: Rc<P::Flash>) {
        self.capsules
            .insert(Index::INFO_FLASH, Capsule::InfoFlash { flash });
    }

    /// Update the HMAC configuration.
    pub fn update_hmac(&mut self, hmac: Rc<P::Hmac>, length: usize) {
        self.capsules
            .insert(Index::HMAC, Capsule::Hmac { hmac, length });
    }

    /// Update the AES configuration.
    pub fn update_aes(&mut self, aes: Rc<P::Aes>, number_of_blocks: usize) {
        self.capsules.insert(
            Index::AES,
            Capsule::Aes {
                aes,
                number_of_blocks,
            },
        );
    }

    pub fn update_gpio(&mut self, pins: Vec<<P::Gpio as crate::Gpio>::PinId>) {
        self.capsules.insert(Index::GPIO, Capsule::Gpio { pins });
    }

    pub fn update_led(&mut self, led_type: LedType, pins: Vec<<P::Gpio as crate::Gpio>::PinId>) {
        self.capsules
            .insert(Index::LED, Capsule::Led { led_type, pins });
    }

    pub fn update_lldb(&mut self, uart: Rc<P::Uart>, baud_rate: usize) {
        self.capsules
            .insert(Index::LLDB, Capsule::Lldb { uart, baud_rate });
    }

    pub fn update_ipc(&mut self) {
        self.capsules.insert(Index::IPC, Capsule::Ipc {});
    }

    /// Update the scheduler configuration.
    pub fn update_scheduler(&mut self, scheduler_type: SchedulerType) {
        self.scheduler = scheduler_type;
    }

    /// Update the stack size.
    pub fn update_stack_size(&mut self, stack_size: usize) {
        if let Some(s) = NonZeroUsize::new(stack_size) {
            self.stack_size = s;
        }
    }

    /// Update the type of syscall filter.
    pub fn update_syscall_filter(&mut self, syscall_filter: SyscallFilterType) {
        self.syscall_filter = syscall_filter
    }

    pub fn update_type(&mut self, ty: impl Into<String>) {
        self.r#type = ty.into();
    }

    /// Remove the console configuration.
    pub fn remove_console(&mut self) {
        self.capsules.remove(&Index::CONSOLE);
    }

    /// Remove the gpio configuration.
    pub fn remove_gpio(&mut self) {
        self.capsules.remove(&Index::GPIO);
    }

    /// Remove the ble configuration.
    pub fn remove_ble(&mut self) {
        self.capsules.remove(&Index::BLE);
    }

    /// Remove the lsm303agr configuration.
    pub fn remove_lsm303agr(&mut self) {
        self.capsules.remove(&Index::LSM303AGR);
    }

    /// Remove the flash configuration.
    pub fn remove_flash(&mut self) {
        self.capsules.remove(&Index::FLASH);
    }

    /// Remove the info flash configuration.
    pub fn remove_info_flash(&mut self) {
        self.capsules.remove(&Index::INFO_FLASH);
    }

    /// Remove the hmac configuration.
    pub fn remove_hmac(&mut self) {
        self.capsules.remove(&Index::HMAC);
    }

    /// Remove the aes configuration.
    pub fn remove_aes(&mut self) {
        self.capsules.remove(&Index::AES);
    }

    /// Remove the LED configuration.
    pub fn remove_led(&mut self) {
        self.capsules.remove(&Index::LED);
    }

    pub fn remove_lldb(&mut self) {
        self.capsules.remove(&Index::LLDB);
    }

    pub fn remove_ipc(&mut self) {
        self.capsules.remove(&Index::IPC);
    }
}
