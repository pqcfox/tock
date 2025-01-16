// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

mod aes;
mod alert_handler;
mod attestation;
mod chip;
mod epmp;
mod flash;
mod flash_memory_protection;
mod gpio;
mod hmac;
mod i2c;
mod pattgen;
mod peripherals;
mod reset_manager;
mod rng;
mod spi;
mod system_reset_controller;
mod timer;
mod uart;
mod usb;
mod watchdog;

pub use chip::*;
