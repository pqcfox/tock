// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! System call interface for generic USB transport layer

use kernel::ErrorCode;
use kernel::ProcessId;
use kernel::grant::{Grant, AllowRoCount, AllowRwCount, UpcallCount};
use kernel::hil::usb;
use kernel::syscall::{CommandReturn, SyscallDriver};

pub const DRIVER_NUM: usize = capsules_core::driver::NUM::UsbUser2 as usize;

#[derive(Default)]
pub struct AppData {

}

const UPCALL_COUNT: u8 = 2;
const ALLOW_RO_COUNT: u8 = 1;
const ALLOW_RW_COUNT: u8 = 1;

type UsbGrant = Grant<
    AppData,
    UpcallCount<UPCALL_COUNT>,
    AllowRoCount<ALLOW_RO_COUNT>,
    AllowRwCount<ALLOW_RW_COUNT>,
>;

pub struct UsbSyscallDriver<
    'a,
    Usb: usb::UsbController<'a>,
> {
    usb: &'a Usb,
    grant: UsbGrant,
}

impl<'a, Usb: usb::UsbController<'a>> UsbSyscallDriver<'a, Usb> {
    pub const fn new(usb: &'a Usb, grant: UsbGrant) -> Self {
        Self {
            usb,
            grant,
        }
    }
}

enum Command {
    DriverExists = 0,
}

impl Command {
    const fn new(command_number: usize) -> Result<Self, ()> {
        const DRIVER_EXISTS_NUMBER: usize = Command::DriverExists as usize;
        match command_number {
            DRIVER_EXISTS_NUMBER => Ok(Command::DriverExists),
            _ => Err(()),
        }
    }
}

impl<'a, Usb: usb::UsbController<'a>> SyscallDriver for UsbSyscallDriver<'a, Usb> {
    fn command(
        &self,
        command_number: usize,
        _argument1: usize,
        _argument2: usize,
        _process_id: ProcessId
    ) -> CommandReturn {
        let command = match Command::new(command_number) {
            Ok(command) => command,
            Err(()) => return CommandReturn::failure(ErrorCode::NOSUPPORT),
        };

        match command {
            Command::DriverExists => CommandReturn::success(),
        }
    }

    fn allocate_grant(&self, process_id: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(process_id, |_, _| {})
    }
}
