// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

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
    DriverExistence,
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
            DRIVER_EXISTENCE_NUMBER => Ok(Command::DriverExistence),
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

/// IDs of all possible upcalls
#[repr(usize)]
enum UpcallId {
    PattGenDone,
    CapsuleUnlocked,
}

impl UpcallId {
    const fn to_usize(self) -> usize {
        // CAST: UpcallId is marked repr(usize)
        self as usize
    }
}

/// The configuration of a channel
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

/// Data to be stored for each process by the Pattern generator capsule
#[derive(Default)]
pub struct AppData {
    channel_config: ChannelConfig,
}

impl AppData {
    /// Return the configured pattern
    ///
    /// # Return value
    ///
    /// The configured pattern
    fn get_pattern(&self) -> &[u32; 2] {
        &self.channel_config.pattern
    }

    /// Configure the pattern
    ///
    /// # Parameters
    ///
    /// + `bottom_half`: the bottom half of the pattern
    /// + `top_half`: the top half of the pattern
    fn set_pattern(&mut self, bottom_half: u32, top_half: u32) {
        self.channel_config.pattern[0] = bottom_half;
        self.channel_config.pattern[1] = top_half;
    }

    /// Return the configured pattern length
    ///
    /// # Return value
    ///
    /// The configured pattern length
    fn get_pattern_length(&self) -> NonZeroUsize {
        self.channel_config.pattern_length
    }

    /// Configure the pattern length
    ///
    /// # Parameters
    ///
    /// + `pattern_length`: the pattern length to be configured
    fn set_pattern_length(&mut self, pattern_length: NonZeroUsize) {
        self.channel_config.pattern_length = pattern_length;
    }

    /// Return the configured pattern repetition count
    ///
    /// # Return value
    ///
    /// The configured pattern repetition count
    fn get_pattern_repetition_count(&self) -> NonZeroUsize {
        self.channel_config.pattern_repetition_count
    }

    /// Configure the pattern repetition count
    ///
    /// # Parameters
    ///
    /// + `pattern_repetition_count`: the pattern repetition count to be configured
    fn set_pattern_repetition_count(&mut self, pattern_repetition_count: NonZeroUsize) {
        self.channel_config.pattern_repetition_count = pattern_repetition_count;
    }

    /// Return the configured predivider
    ///
    /// # Return value
    ///
    /// The configured predivider
    fn get_predivider(&self) -> usize {
        self.channel_config.predivider
    }

    /// Configure the predivider
    ///
    /// # Parameters
    ///
    /// + `predivider`: the predivider to be configured
    fn set_predivider(&mut self, predivider: usize) {
        self.channel_config.predivider = predivider;
    }
}

/// Number of upcalls used by the capsule
const UPCALL_ID_COUNT: u8 = 2;
/// Number of read-only allows used by the capsule
const RO_ALLOW_COUNT: u8 = 0;
/// Number of read-write allows used by the capsule
const RW_ALLOW_COUNT: u8 = 0;

/// Grant used by pattern generator capsule.
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

    /// Extract the pattern length and repetition count from the first argument of a command system
    /// call.
    ///
    /// # Parameter
    ///
    /// + `argument1`: the first argument of the command system call
    ///
    /// # Return value
    ///
    /// + Ok((pattern_length, repetition_count)): the extracted pattern length and repetition count
    /// + Err(()): either pattern length or repetition count is wrong
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

    /// Configure the pattern to be used by the start command.
    ///
    /// # Parameters
    ///
    /// + `bottom_half`: the bottom half of the pattern
    /// + `top_half`: the top half of the pattern
    /// + `process_id`: the identifier of the process that wishes to configure the pattern
    ///
    /// # Return value
    ///
    /// + CommandReturn::success(): the pattern has been successfully configured
    /// + CommandReturn::failure(error): pattern configuration failed with `error`
    fn configure_pattern(
        &self,
        bottom_half: u32,
        top_half: u32,
        process_id: ProcessId,
    ) -> CommandReturn {
        match self.grant.enter(process_id, |app_data, _| {
            app_data.set_pattern(bottom_half, top_half);
            CommandReturn::success()
        }) {
            Err(error) => CommandReturn::from(error),
            Ok(command_return) => command_return,
        }
    }

    /// Configure the pattern parameters.
    ///
    /// # Parameters
    ///
    /// + `pattern_length`: length of the pattern
    /// + `pattern_repetition_count`: pattern repetition count
    /// + `predivider`: clock input predivider
    /// + `process_id`: the identifier of the process that wishes to configure the channel
    ///
    /// # Return value
    ///
    /// + CommandReturn::success(): the pattern parameters have been successfully configured
    /// + CommandReturn::failure(error): pattern configuration failed with `error`
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
    ///
    /// # Return value
    ///
    /// + CommandReturn::success(): pattern generation successfully started
    /// + CommandReturn::failure(ErrorCode::INVAL): either pattern length or pettern repetition
    /// count are invalid
    /// + CommandReturn::failure(ErrorCode::NOMEM): not enough memory available for grant
    /// allocation
    /// + CommandReturn::failure(error): start command failed with `error`
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
    ///
    /// # Parameters
    ///
    /// + `channel`: the channel to be stopped
    ///
    /// # Return value
    ///
    /// + CommandReturn::success(): stop command succeeded
    /// + CommandReturn::failure(error): stop command failed with `error`
    fn stop_command(
        &self,
        channel: <PattGenPeripheral as PattGenHIL<'a>>::Channel,
    ) -> CommandReturn {
        match self.pattgen.stop(channel) {
            Ok(()) => CommandReturn::success(),
            Err(error) => CommandReturn::failure(error),
        }
    }

    /// Check if a process owns capsule's lock
    ///
    /// # Parameters
    ///
    /// + `process_id`: the process that needs to be checked
    ///
    /// # Return value
    ///
    /// + false: `process_id` does not own capsule's lock
    /// + true: `process_id` owns capsule's lock
    fn is_owner(&self, process_id: ProcessId) -> bool {
        match self.get_owner() {
            None => false,
            Some(owner_id) => owner_id == process_id,
        }
    }

    /// Get the current capsule's lock owner
    ///
    /// # Return value
    ///
    /// + None: no process owns the lock
    /// + Some(process_id): `process_id` owns the lock
    fn get_owner(&self) -> Option<ProcessId> {
        let owner_id = self.owner.get()?;

        if let Err(kernel::process::Error::NoSuchApp) = self.grant.enter(owner_id, |_, _| {}) {
            return None;
        }

        Some(owner_id)
    }

    /// Handler of pattgen command
    ///
    /// # Parameters
    ///
    /// + `pattgen_command`: pattern generator command
    /// + `argument1`: the first command argument
    /// + `argument2`: the second command argument
    /// + `process_id`: the process issuing the command
    ///
    /// # Return value
    ///
    /// + CommandReturn::success(): `pattgen_command` succeeded
    /// + CommandReturn::failure(error): `pattgen_command` failed with `error`
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

    /// Handle lock command
    ///
    /// # Parameters
    ///
    /// + `process_id`: the identifier of the process that issued the lock command
    ///
    /// # Return value
    ///
    /// + CommandReturn::success(): the lock command succeeded
    /// + CommandReturn::failure(ErrorCode::BUSY): the capsule's lock is owned by another process
    /// + CommandReturn::failure(ErrorCode::ALREADY): `process_id` already owns the capsule's lock
    fn handle_lock_command(&self, process_id: ProcessId) -> CommandReturn {
        match self.get_owner() {
            Some(owner_id) => {
                let error_code = match owner_id == process_id {
                    false => ErrorCode::BUSY,
                    true => ErrorCode::ALREADY,
                };

                CommandReturn::failure(error_code)
            }
            None => {
                self.owner.set(process_id);
                CommandReturn::success()
            }
        }
    }

    /// Notify all processes waiting to acquire the capsule's lock when an unlock command is
    /// issued.
    ///
    /// The processes are notified through [CapsuleUnlocked] upcall.
    ///
    /// # Parameters
    ///
    /// + `owner_id`: the process that issued the unlock command
    fn notify_unlock(&self, owner_id: ProcessId) {
        for grant in self.grant.iter() {
            if grant.processid() != owner_id {
                grant.enter(|_, kernel_data| {
                    let _ = kernel_data
                        .schedule_upcall(UpcallId::CapsuleUnlocked.to_usize(), (0, 0, 0));
                });
            }
        }
    }

    /// Handle unlock command
    ///
    /// # Parameters
    ///
    /// + `process_id`: the process that issued the unlock command
    ///
    /// # Return value
    ///
    /// + CommandReturn::success(): the unlock command succeeded
    /// + CommandReturn::failure(ErrorCode::BUSY): the capsule's lock is owned by another process
    /// + CommandReturn::failure(ErrorCode::ALREADY): the capsule is already unlocked
    fn handle_unlock_command(&self, process_id: ProcessId) -> CommandReturn {
        match self.get_owner() {
            None => CommandReturn::failure(ErrorCode::ALREADY),
            Some(owner_id) => {
                if owner_id == process_id {
                    self.owner.clear();
                    self.notify_unlock(owner_id);
                    CommandReturn::success()
                } else {
                    CommandReturn::failure(ErrorCode::BUSY)
                }
            }
        }
    }

    /// Handler of locking command
    ///
    /// # Parameters
    ///
    /// + `locking command`: locking command
    /// + `process_id`: the identifier of the process that issued the locking command
    fn handle_locking_command(
        &self,
        locking_command: LockingCommand,
        process_id: ProcessId,
    ) -> CommandReturn {
        match locking_command {
            LockingCommand::Lock => self.handle_lock_command(process_id),
            LockingCommand::Unlock => self.handle_unlock_command(process_id),
        }
    }

    /// Convert a raw command argument to channel
    ///
    /// # Return value
    ///
    /// + Ok(channel): the conversion succeeded
    /// + Err(error): the conversion failed with `error`
    fn get_channel_from_argument(
        argument: usize,
    ) -> Result<
        <PattGenPeripheral as PattGenHIL<'a>>::Channel,
        <<PattGenPeripheral as PattGenHIL<'a>>::Channel as TryFrom<usize>>::Error,
    > {
        <PattGenPeripheral as PattGenHIL<'a>>::Channel::try_from(argument)
    }

    /// Schedule pattgen done upcall
    ///
    /// # Parameters
    ///
    /// + `process_id`: the process to be notified through [PattGenDone] upcall
    /// + `channel`: the channel that finished pattern generation
    fn schedule_pattgen_done_upcall(
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
            Command::DriverExistence => CommandReturn::success(),
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
        if let Some(owner_id) = self.get_owner() {
            self.schedule_pattgen_done_upcall(owner_id, channel);
        }
    }
}
