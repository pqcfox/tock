// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022
// Copyright OxidOS Automotive SRL 2022
//

//! Syscall driver capsule for alert handling on OpenTitan MCUs
//!
//! Usage
//! -----
//!
//! You need a driver that implements the Can trait.
//! ```rust
//! let grant_cap = create_capability!(capabilities::MemoryAllocationCapability);
//! let grant_can = self.board_kernel.create_grant(
//!     capsules::can::CanCapsule::DRIVER_NUM, &grant_cap);
//! let can = capsules::can::CanCapsule::new(
//!    can_peripheral,
//!    grant_can,
//!    tx_buffer,
//!    rx_buffer,
//! );
//!
//! kernel::hil::can::Controller::set_client(can_peripheral, Some(can));
//! kernel::hil::can::Transmit::set_client(can_peripheral, Some(can));
//! kernel::hil::can::Receive::set_client(can_peripheral, Some(can));
//! ```
//!

use capsules_core::driver;
use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::opentitan_alerthandler::OpentTitanAlertHandlerClient;
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::OptionalCell;
use kernel::ErrorCode;
use kernel::ProcessId;

pub const DRIVER_NUM: usize = driver::NUM::OpenTitanAlertHandler as usize;
mod up_calls {
    pub const UPCALL_ALERT_HAPPENED: usize = 0;
    pub const COUNT: u8 = 1;
}

pub struct AlertHandlerCapsule {
    // AlertHandler driver
    processes: Grant<(), UpcallCount<{ up_calls::COUNT }>, AllowRoCount<0>, AllowRwCount<0>>,
    processid: OptionalCell<ProcessId>,
}

impl AlertHandlerCapsule {
    pub fn new(
        grant: Grant<(), UpcallCount<{ up_calls::COUNT }>, AllowRoCount<0>, AllowRwCount<0>>,
    ) -> Self {
        Self {
            processes: grant,
            processid: OptionalCell::empty(),
        }
    }
}

impl SyscallDriver for AlertHandlerCapsule {
    fn command(
        &self,
        command_num: usize,
        _: usize,
        _: usize,
        processid: ProcessId,
    ) -> CommandReturn {
        // save the processid of the first process that syscalls this capsule
        if !self.processid.is_some() {
            self.processid.set(processid);
        }

        match command_num {
            0 => CommandReturn::success(),
            _ => CommandReturn::failure(ErrorCode::NOSUPPORT),
        }
    }

    fn allocate_grant(&self, process_id: ProcessId) -> Result<(), kernel::process::Error> {
        self.processes.enter(process_id, |_, _| {})
    }
}

impl OpentTitanAlertHandlerClient for AlertHandlerCapsule {
    fn alert_happened(&self, alert: u32) {
        // send an upcall to the registered application (`processid`), drop the upcall if it can't be made
        if let Some(processid) = self.processid.get() {
            let _ = self.processes.enter(processid, |_, kernel_data| {
                let _ = kernel_data
                    .schedule_upcall(up_calls::UPCALL_ALERT_HAPPENED, (alert as usize, 0, 0));
            });
        }
    }
}
