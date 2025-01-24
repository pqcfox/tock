// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Implementations for generic LowRISC peripherals.

#![no_std]
#![crate_name = "lowrisc"]
#![crate_type = "rlib"]

#[cfg(feature = "ffi")]
pub use multitop_registers as registers;

pub mod aes;
pub mod aon_timer;
pub mod csrng;
#[cfg(feature = "ffi")]
pub mod ffi;
#[cfg(not(feature = "ffi"))]
pub mod flash_ctrl;
pub mod gpio;
pub mod hmac;
pub mod i2c;
pub mod otbn;
pub mod otp;
pub mod pattgen;
#[cfg(not(feature = "ffi"))]
pub mod registers;
pub mod rsa;
pub mod rv_core_ibex;
pub mod spi_host;
pub mod sysrst_ctrl;
pub mod timer;
pub mod uart;
pub mod usb;
mod utils;
pub mod virtual_otbn;
