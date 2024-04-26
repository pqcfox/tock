// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Pattern generator capsule for OpenTitan.

use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::pattgen::{PattGen as PattGenHIL, PattGenClient};
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::OptionalCell;
use kernel::{ErrorCode, ProcessId};

use core::num::NonZeroUsize;

pub const DRIVER_NUM: usize = capsules_core::driver::NUM::Pattgen as usize;

/// List of commands that control the peripheral
enum PattgenCommand {
    DriverExistence,
    ConfigurePattern,
    ConfigurePatternParams,
    Start,
    Stop,
}

/// List of commands for exclusive access of the capsule
enum LockingCommand {
    Lock,
    Unlock,
}

/// List of all possible commands
enum Command {
    PattgenCommand(PattgenCommand),
    LockingCommand(LockingCommand),
}

/// Association between a number and its corresponding command
enum CommandNumber {
    DriverExistence = 0,
    ConfigurePattern = 1,
    ConfigurePatternParams = 2,
    Start = 3,
    Stop = 4,
    Lock = 5,
    Unlock = 6,
}

impl TryFrom<usize> for Command {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        const DRIVER_EXISTENCE_NUMBER: usize = CommandNumber::DriverExistence as usize;
        const CONFIGURE_PATTERN_NUMBER: usize = CommandNumber::ConfigurePattern as usize;
        const CONFIGURE_PATTERN_PARAMS_NUMBER: usize =
            CommandNumber::ConfigurePatternParams as usize;
        const START_NUMBER: usize = CommandNumber::Start as usize;
        const STOP_NUMBER: usize = CommandNumber::Stop as usize;
        const LOCK_NUMBER: usize = CommandNumber::Lock as usize;
        const UNLOCK_NUMBER: usize = CommandNumber::Unlock as usize;

        match value {
            DRIVER_EXISTENCE_NUMBER => Ok(Command::PattgenCommand(PattgenCommand::DriverExistence)),
            CONFIGURE_PATTERN_NUMBER => {
                Ok(Command::PattgenCommand(PattgenCommand::ConfigurePattern))
            }
            CONFIGURE_PATTERN_PARAMS_NUMBER => Ok(Command::PattgenCommand(
                PattgenCommand::ConfigurePatternParams,
            )),
            START_NUMBER => Ok(Command::PattgenCommand(PattgenCommand::Start)),
            STOP_NUMBER => Ok(Command::PattgenCommand(PattgenCommand::Stop)),
            LOCK_NUMBER => Ok(Command::LockingCommand(LockingCommand::Lock)),
            UNLOCK_NUMBER => Ok(Command::LockingCommand(LockingCommand::Unlock)),
            _ => Err(()),
        }
    }
}

#[repr(usize)]
enum UpcallId {
    PattGenDone,
}

impl UpcallId {
    const fn to_usize(self) -> usize {
        // CAST: UpcallId is marked repr(usize)
        self as usize
    }
}

struct ChannelConfig {
    pattern: [u32; 2],
    pattern_length: NonZeroUsize,
    pattern_repetition_count: NonZeroUsize,
    predivider: usize,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            pattern: [0u32; 2],
            // PANIC: 1 != 0
            pattern_length: NonZeroUsize::new(1).unwrap(),
            // PANIC: 1 != 0
            pattern_repetition_count: NonZeroUsize::new(1).unwrap(),
            predivider: 1,
        }
    }
}

#[derive(Default)]
pub struct AppData {
    channel_config: ChannelConfig,
}

impl AppData {
    fn get_pattern(&self) -> &[u32; 2] {
        &self.channel_config.pattern
    }

    fn set_pattern(&mut self, value1: u32, value2: u32) {
        self.channel_config.pattern[0] = value1;
        self.channel_config.pattern[1] = value2;
    }

    fn get_pattern_length(&self) -> NonZeroUsize {
        self.channel_config.pattern_length
    }

    fn set_pattern_length(&mut self, pattern_length: NonZeroUsize) {
        self.channel_config.pattern_length = pattern_length;
    }

    fn get_pattern_repetition_count(&self) -> NonZeroUsize {
        self.channel_config.pattern_repetition_count
    }

    fn set_pattern_repetition_count(&mut self, pattern_repetition_count: NonZeroUsize) {
        self.channel_config.pattern_repetition_count = pattern_repetition_count;
    }

    fn get_predivider(&self) -> usize {
        self.channel_config.predivider
    }

    fn set_predivider(&mut self, predivider: usize) {
        self.channel_config.predivider = predivider;
    }
}

/// Number of upcalls used by the capsule
const UPCALL_ID_COUNT: u8 = 1;
/// Number of read-only allows used by the capsule
const RO_ALLOW_COUNT: u8 = 0;
/// Number of read-write allows used by the capsule
const RW_ALLOW_COUNT: u8 = 0;

pub type PattGenGrant = Grant<
    AppData,
    UpcallCount<UPCALL_ID_COUNT>,
    AllowRoCount<RO_ALLOW_COUNT>,
    AllowRwCount<RW_ALLOW_COUNT>,
>;

/// Capsule providing userspace access to pattern generator
pub struct PattGen<'a, PattGenPeripheral: PattGenHIL<'a>> {
    pattgen: &'a PattGenPeripheral,
    grant: PattGenGrant,
    owner: OptionalCell<ProcessId>,
}

impl<'a, PattGenPeripheral: PattGenHIL<'a>> PattGen<'a, PattGenPeripheral> {
    /// [PattGen] capsule constructor
    ///
    /// # Parameters
    ///
    /// + `pattgen`: a reference to the underlying peripheral
    /// + `grant`: grant used to store process data
    pub fn new(pattgen: &'a PattGenPeripheral, grant: PattGenGrant) -> Self {
        Self {
            pattgen,
            grant,
            owner: OptionalCell::empty(),
        }
    }

    fn extract_pattern_length_and_repetition_count(
        argument1: usize,
    ) -> Result<(NonZeroUsize, NonZeroUsize), ()> {
        let raw_pattern_length = argument1 & 0xFFFF;
        let raw_pattern_repetition_count = (argument1 & 0xFFFF0000) >> 16;

        let pattern_length = match NonZeroUsize::new(raw_pattern_length) {
            Some(pattern_length) => pattern_length,
            None => return Err(()),
        };

        let pattern_repetition_count = match NonZeroUsize::new(raw_pattern_repetition_count) {
            Some(pattern_repetition_count) => pattern_repetition_count,
            None => return Err(()),
        };

        Ok((pattern_length, pattern_repetition_count))
    }

    fn configure_pattern(&self, value1: u32, value2: u32, process_id: ProcessId) -> CommandReturn {
        match self.grant.enter(process_id, |app_data, _| {
            app_data.set_pattern(value1, value2);
            CommandReturn::success()
        }) {
            Err(error) => CommandReturn::from(error),
            Ok(command_return) => command_return,
        }
    }

    fn configure_pattern_params(
        &self,
        pattern_length: NonZeroUsize,
        pattern_repetition_count: NonZeroUsize,
        predivider: usize,
        process_id: ProcessId,
    ) -> CommandReturn {
        match self.grant.enter(process_id, |app_data, _| {
            app_data.set_pattern_length(pattern_length);
            app_data.set_pattern_repetition_count(pattern_repetition_count);
            app_data.set_predivider(predivider);
            CommandReturn::success()
        }) {
            Err(error) => CommandReturn::from(error),
            Ok(command_return) => command_return,
        }
    }

    /// Starts pattern generation on the given channel
    ///
    /// # Parameters
    ///
    /// + `channel`: the channel that pattern generation must be started
    /// + `ro_allow_id`: the read-only ID of the buffer containing channel's configuration
    /// + `process_id`: the process that initiated the start command
    fn start_command(
        &self,
        channel: <PattGenPeripheral as PattGenHIL<'a>>::Channel,
        process_id: ProcessId,
    ) -> CommandReturn {
        self.grant
            .enter(process_id, |app_data, _| {
                let pattern = app_data.get_pattern();

                let pattern_length =
                    match <PattGenPeripheral as PattGenHIL<'a>>::PatternLength::try_from(
                        app_data.get_pattern_length(),
                    ) {
                        Err(_) => return CommandReturn::failure(ErrorCode::INVAL),
                        Ok(pattern_length) => pattern_length,
                    };

                let pattern_repetition_count =
                    match <PattGenPeripheral as PattGenHIL<'a>>::PatternRepetitionCount::try_from(
                        app_data.get_pattern_repetition_count(),
                    ) {
                        Err(_) => return CommandReturn::failure(ErrorCode::INVAL),
                        Ok(pattern_repetition_count) => pattern_repetition_count,
                    };

                let predivider = app_data.get_predivider();

                match self.pattgen.start(
                    pattern,
                    pattern_length,
                    pattern_repetition_count,
                    predivider,
                    channel,
                ) {
                    Ok(()) => CommandReturn::success(),
                    Err(error) => CommandReturn::failure(error),
                }
            })
            .unwrap_or(CommandReturn::failure(ErrorCode::NOMEM))
    }

    /// Stops pattern generation on the given channel
    fn stop_command(
        &self,
        channel: <PattGenPeripheral as PattGenHIL<'a>>::Channel,
    ) -> CommandReturn {
        match self.pattgen.stop(channel) {
            Ok(()) => CommandReturn::success(),
            Err(error) => CommandReturn::failure(error),
        }
    }

    fn is_owner(&self, process_id: ProcessId) -> bool {
        match self.owner.get() {
            None => false,
            Some(owner_id) => owner_id == process_id,
        }
    }

    fn handle_pattgen_command(
        &self,
        pattgen_command: PattgenCommand,
        argument1: usize,
        argument2: usize,
        process_id: ProcessId,
    ) -> CommandReturn {
        if !self.is_owner(process_id) {
            return CommandReturn::failure(ErrorCode::RESERVE);
        }

        match pattgen_command {
            PattgenCommand::DriverExistence => CommandReturn::success(),
            PattgenCommand::ConfigurePattern => {
                // CAST: u32 == usize on RV32I
                self.configure_pattern(argument1 as u32, argument2 as u32, process_id)
            }
            PattgenCommand::ConfigurePatternParams => {
                let (pattern_length, pattern_repetition_count) =
                    match Self::extract_pattern_length_and_repetition_count(argument1) {
                        Err(()) => return CommandReturn::failure(ErrorCode::INVAL),
                        Ok(tuple) => tuple,
                    };
                let predivider = argument2;
                self.configure_pattern_params(
                    pattern_length,
                    pattern_repetition_count,
                    predivider,
                    process_id,
                )
            }
            PattgenCommand::Start => {
                let channel =
                    match PattGen::<PattGenPeripheral>::get_channel_from_argument(argument1) {
                        Err(_) => return CommandReturn::failure(ErrorCode::INVAL),
                        Ok(channel) => channel,
                    };

                self.start_command(channel, process_id)
            }
            PattgenCommand::Stop => {
                let channel =
                    match PattGen::<PattGenPeripheral>::get_channel_from_argument(argument1) {
                        Err(_) => return CommandReturn::failure(ErrorCode::INVAL),
                        Ok(channel) => channel,
                    };

                self.stop_command(channel)
            }
        }
    }

    fn handle_locking_command(
        &self,
        locking_command: LockingCommand,
        process_id: ProcessId,
    ) -> CommandReturn {
        match locking_command {
            LockingCommand::Lock => {
                if self.owner.is_some() {
                    CommandReturn::failure(ErrorCode::BUSY)
                } else {
                    self.owner.set(process_id);
                    CommandReturn::success()
                }
            }
            LockingCommand::Unlock => match self.owner.get() {
                None => CommandReturn::failure(ErrorCode::ALREADY),
                Some(owner_id) => {
                    if owner_id == process_id {
                        self.owner.clear();
                        CommandReturn::success()
                    } else {
                        CommandReturn::failure(ErrorCode::BUSY)
                    }
                }
            },
        }
    }

    fn get_channel_from_argument(
        argument: usize,
    ) -> Result<
        <PattGenPeripheral as PattGenHIL<'a>>::Channel,
        <<PattGenPeripheral as PattGenHIL<'a>>::Channel as TryFrom<usize>>::Error,
    > {
        <PattGenPeripheral as PattGenHIL<'a>>::Channel::try_from(argument)
    }

    fn schedule_upcall(
        &self,
        process_id: ProcessId,
        channel: <PattGenPeripheral as PattGenHIL<'a>>::Channel,
    ) {
        // Ignore any grant errors. There is not much that can be done about that.
        let _ = self.grant.enter(process_id, |_, kernel_data| {
            let raw_channel: usize = channel.into();
            // Ignore the schedule result. There is not much that can be done about that.
            let _ =
                kernel_data.schedule_upcall(UpcallId::PattGenDone.to_usize(), (raw_channel, 0, 0));
        });
    }
}

/// Provide an interface for userland.
impl<'a, PattGenPeripheral: PattGenHIL<'a>> SyscallDriver for PattGen<'a, PattGenPeripheral> {
    fn command(
        &self,
        command_number: usize,
        argument1: usize,
        argument2: usize,
        process_id: ProcessId,
    ) -> CommandReturn {
        let command = match Command::try_from(command_number) {
            Ok(command) => command,
            Err(()) => return CommandReturn::failure(ErrorCode::NOSUPPORT),
        };

        match command {
            Command::PattgenCommand(pattgen_command) => {
                self.handle_pattgen_command(pattgen_command, argument1, argument2, process_id)
            }
            Command::LockingCommand(locking_command) => {
                self.handle_locking_command(locking_command, process_id)
            }
        }
    }

    fn allocate_grant(&self, process_id: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(process_id, |_, _| {})
    }
}

impl<'a, PattGenPeripheral: PattGenHIL<'a>>
    PattGenClient<<PattGenPeripheral as PattGenHIL<'a>>::Channel>
    for PattGen<'a, PattGenPeripheral>
{
    fn pattgen_done(&self, channel: <PattGenPeripheral as PattGenHIL<'a>>::Channel) {
        if let Some(owner_id) = self.owner.get() {
            self.schedule_upcall(owner_id, channel);
        }
    }
}
