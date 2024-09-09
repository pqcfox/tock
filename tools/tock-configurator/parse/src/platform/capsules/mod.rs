// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

pub mod alarm;
pub use alarm::*;

pub mod console;
pub use console::*;

pub mod led;
pub use led::*;

pub mod spi_controller;
pub use spi_controller::SpiController as SpiCapsule;

pub mod ble_radio;
pub use ble_radio::*;

pub mod lsm303agr;
pub use lsm303agr::*;

pub mod temperature;
pub use temperature::Temperature as TemperatureCapsule;

pub mod rng_capsule;
pub use rng_capsule::*;

// Avoid name conflicts
mod i2c;
pub use i2c::*;

// Avoid name conflicts
mod gpio;
pub use gpio::GPIO;

// Avoid name conflicts
mod hmac;
pub use hmac::HmacCapsule;

// Avoid name conflicts
mod aes;
pub use aes::AesCapsule;

// Avoid name conflicts
mod kv_driver;
pub use kv_driver::KvDriver;
