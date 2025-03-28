// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::registers::top_earlgrey::SPI_DEVICE_BASE_ADDR;
use kernel::utilities::StaticRef;
use lowrisc::registers::spi_device_regs::SpiDeviceRegisters;

pub const SPIDEVICE_BASE: StaticRef<SpiDeviceRegisters> =
    unsafe { StaticRef::new(SPI_DEVICE_BASE_ADDR as *const SpiDeviceRegisters) };
