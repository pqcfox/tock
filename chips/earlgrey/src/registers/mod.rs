// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

pub mod alert_handler_regs;
pub mod ast_regs;
pub mod clkmgr_regs;
pub mod pinmux_regs;
pub mod pwrmgr_regs;
pub mod rstmgr_regs;
pub mod rv_plic_regs;
pub mod sensor_ctrl_regs;

#[cfg(not(feature = "ffi"))]
pub mod top_earlgrey;
#[cfg(feature = "ffi")]
pub use top_earlgrey;

// Import multitop registers for use in top-specific drivers.
pub use lowrisc::registers::{
    adc_ctrl_regs, aes_regs, aon_timer_regs, csrng_regs, edn_regs, entropy_src_regs,
    flash_ctrl_regs, gpio_regs, hmac_regs, i2c_regs, keymgr_regs, kmac_regs, lc_ctrl_regs,
    otbn_regs, otp_ctrl_regs, pattgen_regs, pwm_regs, rom_ctrl_regs, rv_core_ibex_regs,
    rv_timer_regs, spi_device_regs, spi_host_regs, sram_ctrl_regs, sysrst_ctrl_regs, uart_regs,
    usbdev_regs,
};
