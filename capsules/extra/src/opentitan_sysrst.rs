// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Provides userspace applications with access to GPIO pins.
//!
//! GPIOs are presented through a driver interface with synchronous commands
//! and a callback for interrupts.
//!
//! This capsule takes an array of pins to expose as generic GPIOs.
//! Note that this capsule is used for general purpose GPIOs. Pins that are
//! attached to LEDs or buttons are generally wired directly to those capsules,
//! not through this capsule as an intermediary.
//!
//! Usage
//! -----
//!
//! ```rust
//! # use kernel::static_init;
//!
//! let gpio_pins = static_init!(
//!     [Option<&'static sam4l::gpio::GPIOPin>; 4],
//!     [Option<&sam4l::gpio::PB[14]>,
//!      Option<&sam4l::gpio::PB[15]>,
//!      Option<&sam4l::gpio::PB[11]>,
//!      Option<&sam4l::gpio::PB[12]>]);
//! let gpio = static_init!(
//!     capsules::gpio::GPIO<'static, sam4l::gpio::GPIOPin>,
//!     capsules::gpio::GPIO::new(gpio_pins));
//! for maybe_pin in gpio_pins.iter() {
//!     if let Some(pin) = maybe_pin {
//!         pin.set_client(gpio);
//!     }
//! }
//! ```
//!
//! Syscall Interface
//! -----------------
//!
//! - Stability: 2 - Stable
//!
//! ### Commands
//!
//! All GPIO operations are synchronous.
//!
//! Commands control and query GPIO information, namely how many GPIOs are
//! present, the GPIO direction and state, and whether they should interrupt.
//!
//! ### Subscribes
//!
//! The GPIO interface provides only one callback, which is used for pins that
//! have had interrupts enabled.

/// Syscall driver number.
use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};

use kernel::hil::gpio::{Configure, Input, InterruptWithValue, Output};
use kernel::hil::opentitan_sysrst::{OpenTitanSysRstr, OpenTitanSysRstrClient};

use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::OptionalCell;
use kernel::{ErrorCode, ProcessId};

pub const DRIVER_NUM: usize = capsules_core::driver::NUM::OpenTitanSysRst as usize;

/// ### `subscribe_num`
///
/// - `0`: Subscribe to interrupts from all pins with interrupts enabled.
///        The callback signature is `fn(pin_num: usize, pin_state: bool)`
mod upcall {
    pub const COMBO_DETECTED: usize = 1;
    pub const KEY_INTERRUPT: usize = 2;
    pub const COUNT: u8 = 3;
}

pub struct SystemReset<'a, Driver: OpenTitanSysRstr> {
    driver: &'a Driver,
    grants: Grant<(), UpcallCount<{ upcall::COUNT }>, AllowRoCount<0>, AllowRwCount<0>>,
    owning_process: OptionalCell<ProcessId>,
}

impl<'a, Driver: OpenTitanSysRstr> SystemReset<'a, Driver> {
    pub fn new(
        driver: &'a Driver,
        grant: Grant<(), UpcallCount<{ upcall::COUNT }>, AllowRoCount<0>, AllowRwCount<0>>,
    ) -> Self {
        Self {
            driver: driver,
            grants: grant,
            owning_process: OptionalCell::empty(),
        }
    }
}

impl<'a, Driver: OpenTitanSysRstr> SyscallDriver for SystemReset<'a, Driver> {
    fn command(
        &self,
        command_num: usize,
        _data1: usize,
        _data2: usize,
        calling_process: ProcessId,
    ) -> CommandReturn {
        // Check existance (regardless if which process asks)
        if command_num == 0 {
            return CommandReturn::success();
        }

        // determine if `owning_process` is set and it exists
        // determine if the owning process matches the calling process
        let same_proceess_or_empty = self.owning_process.map_or(None, |current_process| {
            self.grants
                .enter(current_process, |_, _| current_process == calling_process)
                .ok()
        });

        match same_proceess_or_empty {
            // the `calling_process` and the `owning_process` are not the same
            Some(false) => return CommandReturn::failure(ErrorCode::RESERVE),
            // the `owning_process` isn't set/doesn't exist, continue execution
            None => self.owning_process.set(calling_process),
            // the  owning process` and the `calling_process` are the same, continue execution
            Some(true) => {}
        }

        match command_num {
            // get input pins state
            1 => {
                let pin_state = self.driver.get_input_state();
                CommandReturn::success_u32(pin_state.get())
            }
            _ => CommandReturn::failure(ErrorCode::NOSUPPORT),
        }
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
        self.grants.enter(processid, |_, _| {})
    }
}

impl<'a, Driver: OpenTitanSysRstr> OpenTitanSysRstrClient for SystemReset<'a, Driver> {
    fn combo_detected(
        &self,
        input_pin_state: kernel::hil::opentitan_sysrst::SRCInputPinStatus,
        combodetector_id: kernel::hil::opentitan_sysrst::SRCComboDetectorId,
    ) {
        // schedule a COMBO_DETECTED upcall with
        // * r0 = combo detector id
        // * r1 = input pin state in the order defined by `SRCInputPinState`
        // * r2 = 0
        let result = self.owning_process.map(|pid| {
            self.grants.enter(pid, |_app, upcalls| {
                upcalls.schedule_upcall(
                    upcall::COMBO_DETECTED,
                    (combodetector_id as usize, input_pin_state.get() as usize, 0),
                )
            })
        });

        kernel::debug_verbose!("{:?} {:?}", combodetector_id as usize, input_pin_state);

        // no error handling, upcall scheduling will not be retried if an issue appears
        match result {
            // when the upcall was successful
            Some(Ok(Ok(()))) => {}
            // if the upcall coudln't be made (the owning process is registered) (`.schedule upcall`` failed)
            Some(Ok(Err(_err))) => {}
            // if the grant is not available (`.enter` failed)
            Some(Err(_err)) => {}
            // if the owning process is not registered
            None => {}
        }
    }

    fn key_interrupt(
        &self,
        l2h: kernel::hil::opentitan_sysrst::SRCInputPinStatus,
        h2l: kernel::hil::opentitan_sysrst::SRCInputPinStatus,
    ) {
        // schedule a KEY_INTERRUPT upcall with
        // * r0 = keys where a L2H transition was detected
        // * r1 = keys where a H2L transition was detected
        // * r2 = 0
        let result = self.owning_process.map(|pid| {
            self.grants.enter(pid, |_app, upcalls| {
                upcalls.schedule_upcall(
                    upcall::KEY_INTERRUPT,
                    (l2h.get() as usize, h2l.get() as usize, 0),
                )
            })
        });

        // no error handling, upcall scheduling will not be retried if an issue appears
        match result {
            // when the upcall was successful
            Some(Ok(Ok(()))) => {}
            // if the upcall coudln't be made (the owning process is registered) (`.schedule upcall`` failed)
            Some(Ok(Err(_err))) => {}
            // if the grant is not available (`.enter` failed)
            Some(Err(_err)) => {}
            // if the owning process is not registered
            None => {}
        }
    }
}
