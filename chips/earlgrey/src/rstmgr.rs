// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header has to be included to be able to submit it to Tock
// It is up to ZeroRISC to decide if it keeps this header or not

use crate::registers::{
    rstmgr_regs::{
        RstmgrRegisters, ALERT_INFO, ALERT_INFO_ATTR, ALERT_INFO_CTRL, ALERT_REGWEN, ALERT_TEST,
        CPU_INFO, CPU_INFO_ATTR, CPU_INFO_CTRL, CPU_REGWEN, RESET_INFO, RESET_REQ, SW_RST_CTRL_N,
        SW_RST_REGWEN,
    },
    top_earlgrey::RSTMGR_AON_BASE_ADDR,
};
use kernel::{
    hil::{
        reset_managment::{ResetManagment, ResetReason},
        retention_ram::CreatorRetentionRam,
    },
    utilities::{
        registers::interfaces::{ReadWriteable, Readable, Writeable},
        StaticRef,
    },
};

pub(crate) const RSTMGR_BASE: StaticRef<RstmgrRegisters> =
    unsafe { StaticRef::new(RSTMGR_AON_BASE_ADDR as *const RstmgrRegisters) };

pub const RSTMGR_ALERT_INFO_DUMP_SIZE: usize = 9;
pub const RSTMGR_CPU_INFO_DUMP_SIZE: usize = 8;
pub const RSTMGR_SW_RESET_MAGIC: u32 = 0x6;

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum RstMgrReason {
    /// register RESET_INFO didn't contain any set bits
    None = 0,

    /// POR: Indicates when a device has reset due to power up.
    PoR = 0b0000_0001,

    /// Low_Power_Exit: Indicates when a device has reset due low power exit
    LowPowerExit = 0b0000_0010,

    /// SW_RESET: Indicates when a device has reset due to RESET_REQ.
    SoftwareReset = 0b0000_0100,

    /// sysrst_ctrl_aon: OpenTitan reset request to rstmgr (running on AON clock)
    SystemResetController = 0b0000_1000,

    /// aon_timer_aon: watchdog reset request
    AonTimer = 0b0001_0000,

    /// pwrmgr_aon: main power glitch reset request
    PowerManager = 0b0010_0000,

    /// alert_handler: escalation reset request
    AlertHandler = 0b0100_0000,

    ///rv_dm: non-debug-module reset request
    Debug = 0b1000_0000,

    /// multiple bits are set, probably register was not cleared between resets
    MultipleReasons(u8),
}

/// convert HW register content into MCU specific `ResetManagerReason`
impl TryFrom<u32> for RstMgrReason {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0b0000_0000 => Ok(RstMgrReason::None),
            0b0000_0001 => Ok(RstMgrReason::PoR),
            0b0000_0010 => Ok(RstMgrReason::LowPowerExit),
            0b0000_0100 => Ok(RstMgrReason::SoftwareReset),
            0b0000_1000 => Ok(RstMgrReason::SystemResetController),
            0b0001_0000 => Ok(RstMgrReason::AonTimer),
            0b0010_0000 => Ok(RstMgrReason::PowerManager),
            0b0100_0000 => Ok(RstMgrReason::AlertHandler),
            0b1000_0000 => Ok(RstMgrReason::Debug),
            _ => Err(()),
        }
    }
}

/// convert MCU specific `ResetManagerReason` into universal `ResetReason`
impl From<RstMgrReason> for Option<ResetReason> {
    fn from(value: RstMgrReason) -> Self {
        match value {
            RstMgrReason::None => None,
            RstMgrReason::PoR => Some(ResetReason::PowerOnReset),
            RstMgrReason::LowPowerExit => Some(ResetReason::LowPowerExit),
            RstMgrReason::SoftwareReset => Some(ResetReason::SoftwareRequest),
            RstMgrReason::SystemResetController => Some(ResetReason::PeripheralRequest(0)),
            RstMgrReason::AonTimer => Some(ResetReason::Watchdog),
            RstMgrReason::PowerManager => Some(ResetReason::VoltageFault),
            RstMgrReason::AlertHandler => Some(ResetReason::PeripheralRequest(1)),
            RstMgrReason::Debug => Some(ResetReason::Debug),
            RstMgrReason::MultipleReasons(reason) => Some(ResetReason::Unknown(reason.into())),
        }
    }
}

/// bit position for peripherals that can be reset by RstMgr
#[derive(Copy, Clone)]
pub enum RstMgrPeripherals {
    SpiDevice = 0,
    SpiHost0 = 1,
    SpiHost1 = 2,
    Usb = 3,
    UsbAon = 4,
    I2c0 = 5,
    I2c1 = 6,
    I2c2 = 7,
}

pub enum RstMgrTriggerableFault {
    ConsistencyFault,
    FatalFault,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum RstMgrDetectedError {
    Any = 0b111,
    FsmErr = 0b100,
    ResetConsistentcyErr = 0b010,
    RegisterIntegrityErr = 0b001,
}

pub struct RstMgr {
    registers: StaticRef<RstmgrRegisters>,
}

impl RstMgr {
    pub fn new() -> Self {
        Self {
            registers: RSTMGR_BASE,
        }
    }

    // MCU RESET

    /// Return the reason the MCU was reset. This funciton should be called once at startup in order to cache the reason and clear the RESET_INFO register. Subsequent calls will return the cached value, not the value from RESET_INFO register.
    pub fn read_reset_reason(&self) -> RstMgrReason {
        // try to convert the register content (u32) to ResetManagerReason enum
        let raw_register = self.registers.reset_info.get();
        let reset_info: Result<RstMgrReason, ()> = raw_register.try_into();

        // conversion might fail because:
        // 1. an unknown bit is set (can't happen on current chip)
        // 2. multiple reasons are detected
        let reset_reason = reset_info.unwrap_or(RstMgrReason::MultipleReasons(raw_register as u8));

        // save the reset_reason and clear the RESET_INFO register
        self.clear_reset_reason();
        reset_reason
    }

    /// clear the content of RESET_INFO
    pub fn clear_reset_reason(&self) {
        // RESET_INFO is write 1 to clear
        self.registers.reset_info.write(
            RESET_INFO::POR.val(1)
                + RESET_INFO::LOW_POWER_EXIT.val(1)
                + RESET_INFO::SW_RESET.val(1)
                + RESET_INFO::HW_REQ.val(0b11111),
        );
    }

    /// trigger a software reset by writing the magic value into RESET_REQ register
    pub fn do_software_reset(&self) {
        self.registers
            .reset_req
            .write(RESET_REQ::VAL.val(RSTMGR_SW_RESET_MAGIC));
    }

    // ALERT INFO

    /// Reads the entire alert info crash dump and stores it in `alert_info`.
    pub fn dump_alert_info(&self) -> (usize, [u32; RSTMGR_ALERT_INFO_DUMP_SIZE]) {
        self.allow_new_alert_info();
        // the actual crash dump size (can be smaller than `RSTMGR_ALERT_INFO_DUMP_SIZE`)
        let actual_size = self
            .registers
            .alert_info_attr
            .read(ALERT_INFO_ATTR::CNT_AVAIL) as usize;

        let mut data: [u32; RSTMGR_ALERT_INFO_DUMP_SIZE] = [0; RSTMGR_ALERT_INFO_DUMP_SIZE];
        for idx in 0..actual_size - 1 {
            // set the index of the 32bit data segment to be read
            self.registers
                .alert_info_ctrl
                .modify(ALERT_INFO_CTRL::INDEX.val(idx as u32));
            // get the alert info crash
            let content = self.registers.alert_info.read(ALERT_INFO::VALUE);
            data[idx] = content;
        }
        self.lock_new_alert_info();
        (actual_size, data)
    }

    /// lock (from HW POV) ALERT_INFO registers so that SW can read from them without HW being able to overwrite these register
    fn lock_new_alert_info(&self) {
        // unlock ALERT_INFO registers for writes
        self.registers.alert_regwen.modify(ALERT_REGWEN::EN.val(1));
        // do not let hardware overwrite these registers until fn finishes reading
        self.registers
            .alert_info_ctrl
            .modify(ALERT_INFO_CTRL::EN.val(0));
    }

    /// unlock ALERT_INFO registers after this module finished reading from them. Let HW overwrite values
    fn allow_new_alert_info(&self) {
        // let hardware overwrite these registers (in case of another alert)
        self.registers
            .alert_info_ctrl
            .modify(ALERT_INFO_CTRL::EN.val(1));
        // lock down ALERT_INFO registers for writes
        self.registers.alert_regwen.modify(ALERT_REGWEN::EN.val(1));
    }

    // CPU RESET INFO

    /// Reads the entire alert info crash dump and stores it in `alert_info`.
    pub fn dump_cpu_reset_info(&self) -> (usize, [u32; RSTMGR_CPU_INFO_DUMP_SIZE]) {
        self.unlock_cpu_dump_reset_info();
        // the actual crash dump size (can be smaller than `RSTMGR_PARAM_IDX_WIDTH`)
        let actual_size = self.registers.cpu_info_attr.read(CPU_INFO_ATTR::CNT_AVAIL) as usize;

        let mut data: [u32; RSTMGR_CPU_INFO_DUMP_SIZE] = [0; RSTMGR_CPU_INFO_DUMP_SIZE];
        for idx in 0..actual_size - 1 {
            // set the index of the 32bit data segment to be read
            self.registers
                .cpu_info_ctrl
                .modify(CPU_INFO_CTRL::INDEX.val(idx as u32));
            // get the alert info crash
            let content = self.registers.cpu_info.read(CPU_INFO::VALUE);
            data[idx] = content;
        }
        self.lock_cpu_dump_reset_info();
        (actual_size, data)
    }

    /// unlock CPU_INFO registers so that this module can read from them. Block HW from overwriting values
    fn unlock_cpu_dump_reset_info(&self) {
        // unlock CPU_INFO registers for writes
        self.registers.cpu_regwen.modify(CPU_REGWEN::EN.val(1));
        // do not let hardware overwrite these registers until fn finishes reading
        self.registers
            .cpu_info_ctrl
            .modify(CPU_INFO_CTRL::EN.val(0));
    }

    /// lock CPU_INFO registers after this module finished reading from them. Let HW overwrite values
    fn lock_cpu_dump_reset_info(&self) {
        // let hardware overwrite these registers (in case of another alert)
        self.registers
            .cpu_info_ctrl
            .modify(CPU_INFO_CTRL::EN.val(1));
        // lock down CPU_INFO registers for writes
        self.registers.cpu_regwen.modify(CPU_REGWEN::EN.val(0));
    }

    /// PERIPHERAL RESET

    /// trigger the reset of the specified peripheral in `peripheral`
    /// # Returns
    /// * Ok() -if the reset register is not locked and the reset action should be succesful
    /// * Err() - if the reset register is locked and the reset action can't be done
    pub fn reset_peripheral(&self, peripheral: RstMgrPeripherals) -> Result<(), ()> {
        let peripheral_index = peripheral as usize;
        if !self.is_locked_peripheral_reset(peripheral) {
            self.registers.sw_rst_ctrl_n[peripheral_index].write(SW_RST_CTRL_N::VAL_0.val(1));
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn lock_peripheral_reset(&self, peripheral: RstMgrPeripherals) {
        let peripheral_index = peripheral as usize;
        self.registers.sw_rst_regwen[peripheral_index].write(SW_RST_REGWEN::EN_0.val(1));
    }

    pub fn is_locked_peripheral_reset(&self, peripheral: RstMgrPeripherals) -> bool {
        let peripheral_index = peripheral as usize;
        self.registers.sw_rst_regwen[peripheral_index].read(SW_RST_REGWEN::EN_0) != 0
    }

    // Alerts & errors

    /// trigger the generation of an alert using `TEST_ALERT` register
    pub fn trigger_fault(&self, fault: RstMgrTriggerableFault) {
        match fault {
            RstMgrTriggerableFault::FatalFault => {
                self.registers
                    .alert_test
                    .modify(ALERT_TEST::FATAL_FAULT.val(1));
            }
            RstMgrTriggerableFault::ConsistencyFault => {
                self.registers
                    .alert_test
                    .modify(ALERT_TEST::FATAL_CNSTY_FAULT.val(1));
            }
        }
    }

    /// determine if the provided error flag from `ERR_CODE` is raised.
    /// calling this function with  `error = RstMgrDetectedError::Any` will return true if any of the flags are raised
    pub fn is_error_detected(&self, error: RstMgrDetectedError) -> bool {
        let register_content = self.registers.err_code.get();
        let error_mask = error as u32;
        (register_content ^ error_mask) == 0
    }

    /// read Reset Reason from RetentionRAM (stored there by ROM_EXT)
    pub fn get_rr_from_rram(
        creator_ram: &impl CreatorRetentionRam<Data = u32, ID = usize>,
    ) -> Option<ResetReason> {
        // Read reset reason from retention SRAM := offset 0 in creator region, overall byte offset
        // 4 (u32 offset 1).
        let raw_value = creator_ram.read(1).ok()?;
        match TryInto::<RstMgrReason>::try_into(raw_value) {
            Ok(reason) => reason.into(),
            Err(()) => None,
        }
    }
}

impl ResetManagment for RstMgr {
    type ResetInfo = [u32; RSTMGR_ALERT_INFO_DUMP_SIZE + RSTMGR_CPU_INFO_DUMP_SIZE + 2];

    fn trigger_system_reset(&self) {
        self.do_software_reset();
    }

    fn reset_reason(&self) -> Option<kernel::hil::reset_managment::ResetReason> {
        self.read_reset_reason().into()
    }

    /// return the concatenation of `cpu_reset_info` and `alert_info`. The content of this array is not ment for the MCU to interpret during runtime but rather to be store and displayed for later interpretation by a human. Actual content might change.
    /// According to <https://opentitan.org/book/hw/ip/rv_core_ibex/doc/theory_of_operation.html#crash-dump-collection> the content of this data is:
    ///
    /// | bits | description |
    /// | ----- | -- |
    /// | \[0\] | size of the following section (reset_info) |
    /// | \[1\] |   The last exception address (mtval) |
    /// | \[2\] |   The last exception PC (mepc) |
    /// | \[3\] |   The last data access address |
    /// | \[4\] |   The next PC |
    /// | \[5\] |   The current PC |
    /// | \[6\] |   The previous exception address (mtval) |
    /// | \[7\] |   The previous exception PC (mepc) |
    /// | \[8\] |   (MSB) Previous state valid indication |
    /// | \[9\] | size of the following section (alert_info) |
    /// | \[10..18\] | ? |
    fn get_reset_info_dump(&self) -> Option<Self::ResetInfo> {
        let reset_info = self.dump_cpu_reset_info();
        let alert_info = self.dump_alert_info();
        let mut to_return = [0u32; 1 + RSTMGR_CPU_INFO_DUMP_SIZE + 1 + RSTMGR_ALERT_INFO_DUMP_SIZE];
        to_return[0] = reset_info.0 as u32; // size (in 32bit words) of reset_info section
        to_return[1..1 + RSTMGR_CPU_INFO_DUMP_SIZE].copy_from_slice(&reset_info.1);
        to_return[1 + RSTMGR_CPU_INFO_DUMP_SIZE] = alert_info.0 as u32; // size (in 32bit words) of of alert_info section
        to_return[2 + RSTMGR_CPU_INFO_DUMP_SIZE..].copy_from_slice(&alert_info.1);
        Some(to_return)
    }
}
