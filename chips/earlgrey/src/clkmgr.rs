// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Provides access to determine the clock status, enable and disable clocks and
//!  clock measurement checks, get and clear errors.
//!
//!
#[cfg(feature = "clkmgr_tests")]
use kernel::debug;
use kernel::utilities::registers::interfaces::{ReadWriteable, Readable, Writeable};
use kernel::utilities::StaticRef;
use kernel::ErrorCode;

use crate::registers::clkmgr_regs::{
    ClkmgrRegisters, CLK_ENABLES, CLK_HINTS, CLK_HINTS_STATUS, EXTCLK_CTRL, EXTCLK_CTRL_REGWEN,
    EXTCLK_STATUS, FATAL_ERR_CODE, IO_DIV2_MEAS_CTRL_EN, IO_DIV2_MEAS_CTRL_SHADOWED,
    IO_DIV4_MEAS_CTRL_EN, IO_DIV4_MEAS_CTRL_SHADOWED, IO_MEAS_CTRL_EN, IO_MEAS_CTRL_SHADOWED,
    JITTER_ENABLE, JITTER_REGWEN, MAIN_MEAS_CTRL_EN, MAIN_MEAS_CTRL_SHADOWED, MEASURE_CTRL_REGWEN,
    RECOV_ERR_CODE, USB_MEAS_CTRL_EN, USB_MEAS_CTRL_SHADOWED,
};

use crate::registers::top_earlgrey::CLKMGR_AON_BASE_ADDR;

pub struct Clkmgr {
    registers: StaticRef<ClkmgrRegisters>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExtClkState {
    ExtClkOnHighSpeed,
    ExtClkOnLowSpeed,
    ExtClkOff,
    ExtClkError,
}

#[derive(Clone, Copy, Debug)]
pub enum GateableClk {
    UsbPeri,
    IoPeri,
    IoDiv2Peri,
    IoDiv4Peri,
    TransClkMainOTBN,
    TransClkMainKMAC,
    TransClkMainHMAC,
    TransClkMainAES,
}

#[derive(Clone, Copy, Debug)]
pub enum MeasCtrlClk {
    Io,
    IoDiv2,
    IoDiv4,
    MainClk,
    UsbClk,
}

#[derive(Clone, Copy, Debug)]
pub enum RecovErr {
    UsbTimeoutErr,
    MainTimeoutErr,
    IoDiv4TimeoutErr,
    IoDiv2TimeoutErr,
    IoTimeoutErr,
    UsbMeasureErr,
    MainMeasureErr,
    IoDiv4MeasureErr,
    IoDiv2MeasureErr,
    IoMeasureErr,
    ShadowUpdateErr,
}

#[derive(Clone, Copy, Debug)]
pub enum FatalErr {
    ShadowStorageErr,
    IdleCnt,
    RegIntg,
}

const MULTI_BIT_BOOL_4TRUE: u32 = 0x6;
const MULTI_BIT_BOOL_4FALSE: u32 = 0x9;

pub const CLK_MGR_BASE: StaticRef<ClkmgrRegisters> =
    unsafe { StaticRef::new(CLKMGR_AON_BASE_ADDR as *const ClkmgrRegisters) };

#[cfg(feature = "clkmgr_tests")]
/// Test helper that takes a test text describing the test and a pass criteria for the test itself.
///
/// We do not print to the debug buffer anything unless it's a fail to prevent the debug buffer filling
///  with positive feedback. The buffer is limited because we run before the system is fully running and
/// the debugger manages to flush the data.
fn test_helper(test_text: &str, f: impl Fn() -> bool) -> bool {
    static mut TEST_ID: usize = 0;
    unsafe {
        TEST_ID += 1;
        if f() {
            // Keep test success silent, we don't want to fill the buffer if everything is OK!
            true
        } else {
            debug!("*   Test No. {} Failed! : {}", TEST_ID, test_text);
            false
        }
    }
}

impl Clkmgr {
    pub fn new() -> Self {
        Self {
            registers: CLK_MGR_BASE,
        }
    }

    /// Set the extclk state while spin waiting for a certain number of loops.
    ///
    /// The  'req_state' represents the requested state and 'timeout' represents number of spinloops that it should wait for.
    /// Since at this point of initiaization, a lot of times the clocks are not availalble, this is just a number of for loop
    /// iterations. This gives you the option of setting and upper ceiling or not waiting at all, in case you want the clock
    /// request to be set but do not want to wait around for the resolution, you might have other work to do than busy waiting
    /// during the init phase.
    ///
    /// The return is a 'Result<(), ErrorCode>'  that tells you if the clock has been confirmed to have entered the
    /// requested state in case it returns 'Ok()'. it returns 'false'.
    pub fn set_extclk(&self, req_state: ExtClkState, timeout: usize) -> Result<(), ErrorCode> {
        // Return error if the lock is already set, it's sticky and will only get cleared on reset.
        if self.is_extclk_setting_locked() {
            return Err(ErrorCode::FAIL);
        }

        let (extclk_en, extclk_hispeed, feedback_expected) = match req_state {
            ExtClkState::ExtClkOnHighSpeed => (
                EXTCLK_CTRL::SEL.val(MULTI_BIT_BOOL_4TRUE),
                EXTCLK_CTRL::HI_SPEED_SEL.val(MULTI_BIT_BOOL_4TRUE),
                MULTI_BIT_BOOL_4TRUE,
            ),
            ExtClkState::ExtClkOnLowSpeed => (
                EXTCLK_CTRL::SEL.val(MULTI_BIT_BOOL_4TRUE),
                EXTCLK_CTRL::HI_SPEED_SEL.val(MULTI_BIT_BOOL_4FALSE),
                MULTI_BIT_BOOL_4TRUE,
            ),
            ExtClkState::ExtClkOff => (
                EXTCLK_CTRL::SEL.val(MULTI_BIT_BOOL_4FALSE),
                EXTCLK_CTRL::HI_SPEED_SEL.val(MULTI_BIT_BOOL_4FALSE),
                MULTI_BIT_BOOL_4FALSE,
            ),
            _ => return Err(ErrorCode::INVAL),
        };
        self.registers
            .extclk_ctrl_regwen
            .modify(EXTCLK_CTRL_REGWEN::EN::SET);

        self.registers.extclk_ctrl.write(extclk_en + extclk_hispeed);

        for _ in 0..timeout {
            if self.registers.extclk_status.read(EXTCLK_STATUS::ACK) == feedback_expected {
                return Ok(());
            }
        }
        Err(ErrorCode::BUSY)
    }

    /// Returns the state the external clock is in.
    pub fn get_extclk_state(&self) -> ExtClkState {
        match (
            self.registers.extclk_ctrl.read(EXTCLK_CTRL::SEL),
            self.registers.extclk_ctrl.read(EXTCLK_CTRL::HI_SPEED_SEL),
            self.registers.extclk_status.read(EXTCLK_STATUS::ACK),
        ) {
            (MULTI_BIT_BOOL_4TRUE, MULTI_BIT_BOOL_4TRUE, MULTI_BIT_BOOL_4TRUE) => {
                ExtClkState::ExtClkOnHighSpeed
            }
            (MULTI_BIT_BOOL_4TRUE, MULTI_BIT_BOOL_4FALSE, MULTI_BIT_BOOL_4TRUE) => {
                ExtClkState::ExtClkOnLowSpeed
            }
            (MULTI_BIT_BOOL_4FALSE, _, _) => ExtClkState::ExtClkOff,
            (_, _, _) => ExtClkState::ExtClkError,
        }
    }

    /// Lock the extclk setting. This is a sticky lock and can only be removed by reset.
    pub fn lock_extclk_setting(&self) {
        self.registers
            .extclk_ctrl_regwen
            .modify(EXTCLK_CTRL_REGWEN::EN::CLEAR);
    }

    /// Get the the extclk lock setting. Returns 'true' if locked, false otherwise.
    pub fn is_extclk_setting_locked(&self) -> bool {
        match self
            .registers
            .extclk_ctrl_regwen
            .read(EXTCLK_CTRL_REGWEN::EN)
        {
            0x1 => false,
            _ => true,
        }
    }

    /// Sets the state the clock jitter.
    ///
    /// The 'jitter_enabled' argument is the request of the jitter setting
    pub fn set_clk_jitter(&self, jitter_enabled: bool) {
        self.registers.jitter_regwen.write(JITTER_REGWEN::EN::SET);

        self.registers
            .jitter_enable
            .write(JITTER_ENABLE::VAL.val(match jitter_enabled {
                true => MULTI_BIT_BOOL_4TRUE,
                false => MULTI_BIT_BOOL_4FALSE,
            }));

        self.registers.jitter_regwen.write(JITTER_REGWEN::EN::CLEAR);
    }

    /// Gets the state the clock jitter.
    pub fn is_clk_jitter_enabled(&self) -> bool {
        !matches!(
            self.registers.jitter_enable.read(JITTER_ENABLE::VAL),
            MULTI_BIT_BOOL_4FALSE
        )
    }
    /// Requests the enabling state of the speciffic clock.
    ///
    /// The 'clk' argument selects the clock the requests is addressed to. The 'req' argument says the
    /// state that is requested, where 'true' is enabled and 'false' is disabled
    ///
    /// Returns true if the clock enable has succeeded and false if the request did not succeed.
    pub fn set_clk_enable(&self, clk: GateableClk, req: bool) -> bool {
        if self.is_clk_enabled(clk) != req {
            match clk {
                GateableClk::UsbPeri
                | GateableClk::IoPeri
                | GateableClk::IoDiv2Peri
                | GateableClk::IoDiv4Peri => {
                    let bit = match clk {
                        GateableClk::UsbPeri => CLK_ENABLES::CLK_USB_PERI_EN,
                        GateableClk::IoPeri => CLK_ENABLES::CLK_IO_PERI_EN,
                        GateableClk::IoDiv2Peri => CLK_ENABLES::CLK_IO_DIV2_PERI_EN,
                        GateableClk::IoDiv4Peri => CLK_ENABLES::CLK_IO_DIV4_PERI_EN,
                        _ => unreachable!(),
                    };
                    self.registers.clk_enables.modify(bit.val(req as u32));
                }
                GateableClk::TransClkMainOTBN
                | GateableClk::TransClkMainKMAC
                | GateableClk::TransClkMainHMAC
                | GateableClk::TransClkMainAES => {
                    let bit = match clk {
                        GateableClk::TransClkMainOTBN => CLK_HINTS::CLK_MAIN_OTBN_HINT,
                        GateableClk::TransClkMainKMAC => CLK_HINTS::CLK_MAIN_KMAC_HINT,
                        GateableClk::TransClkMainHMAC => CLK_HINTS::CLK_MAIN_HMAC_HINT,
                        GateableClk::TransClkMainAES => CLK_HINTS::CLK_MAIN_AES_HINT,
                        _ => unreachable!(),
                    };
                    self.registers.clk_hints.modify(bit.val(req as u32));
                }
            }
            self.is_clk_enabled(clk)
        } else {
            true
        }
    }

    /// Gets the enabling state of the speciffic clock.
    ///
    /// The 'clk' argument selects the clock it wants to query the state of.
    ///
    /// Returns true if the clock is enabled and false if the clock is not enabled.
    pub fn is_clk_enabled(&self, clk: GateableClk) -> bool {
        match clk {
            GateableClk::UsbPeri
            | GateableClk::IoPeri
            | GateableClk::IoDiv2Peri
            | GateableClk::IoDiv4Peri => {
                let bit = match clk {
                    GateableClk::UsbPeri => CLK_ENABLES::CLK_USB_PERI_EN,
                    GateableClk::IoPeri => CLK_ENABLES::CLK_IO_PERI_EN,
                    GateableClk::IoDiv2Peri => CLK_ENABLES::CLK_IO_DIV2_PERI_EN,
                    GateableClk::IoDiv4Peri => CLK_ENABLES::CLK_IO_DIV4_PERI_EN,
                    _ => unreachable!(),
                };
                self.registers.clk_enables.is_set(bit)
            }
            GateableClk::TransClkMainOTBN
            | GateableClk::TransClkMainKMAC
            | GateableClk::TransClkMainHMAC
            | GateableClk::TransClkMainAES => {
                let bit = match clk {
                    GateableClk::TransClkMainOTBN => CLK_HINTS_STATUS::CLK_MAIN_OTBN_VAL,
                    GateableClk::TransClkMainKMAC => CLK_HINTS_STATUS::CLK_MAIN_KMAC_VAL,
                    GateableClk::TransClkMainHMAC => CLK_HINTS_STATUS::CLK_MAIN_HMAC_VAL,
                    GateableClk::TransClkMainAES => CLK_HINTS_STATUS::CLK_MAIN_AES_VAL,
                    _ => unreachable!(),
                };
                self.registers.clk_hints_status.is_set(bit)
            }
        }
    }

    /// Requests the state of the speciffic clock measurement control.
    ///
    /// The 'clk' argument selects the clock it wants to set the measurement control state for. The 'en' is the state
    /// of the measurement control itself, 'lo' is the low range of the clock measurement supervision and 'hi' is the
    /// high range of the clock mesurement supervision
    ///
    /// Returns an Result that is () if the request succeded and ErrorCode if an error happened during the setting.
    pub fn set_clk_meas_ctrl(
        &self,
        clk: MeasCtrlClk,
        en: bool,
        lo: u32,
        hi: u32,
    ) -> Result<(), ErrorCode> {
        if self.is_locked_meas_ctrl_setting() {
            return Err(ErrorCode::FAIL);
        }

        let set_req = match en {
            true => MULTI_BIT_BOOL_4TRUE,
            false => MULTI_BIT_BOOL_4FALSE,
        };

        // Check that the measurement control limits fit in the register boundries.
        let lo_req = if lo < 0x400 {
            lo
        } else {
            return Err(ErrorCode::INVAL);
        };

        let hi_req = if hi < 0x400 {
            hi
        } else {
            return Err(ErrorCode::INVAL);
        };

        self.registers
            .measure_ctrl_regwen
            .write(MEASURE_CTRL_REGWEN::EN::SET);

        match clk {
            MeasCtrlClk::Io => {
                self.registers
                    .io_meas_ctrl_en
                    .write(IO_MEAS_CTRL_EN::EN.val(set_req));
                self.registers.io_meas_ctrl_shadowed.write(
                    IO_MEAS_CTRL_SHADOWED::LO.val(lo_req) + IO_MEAS_CTRL_SHADOWED::HI.val(hi_req),
                );
                self.registers.io_meas_ctrl_shadowed.write(
                    IO_MEAS_CTRL_SHADOWED::LO.val(lo_req) + IO_MEAS_CTRL_SHADOWED::HI.val(hi_req),
                );
            }
            MeasCtrlClk::IoDiv2 => {
                self.registers
                    .io_div2_meas_ctrl_en
                    .write(IO_DIV2_MEAS_CTRL_EN::EN.val(set_req));
                self.registers.io_div2_meas_ctrl_shadowed.write(
                    IO_DIV2_MEAS_CTRL_SHADOWED::LO.val(lo_req)
                        + IO_DIV2_MEAS_CTRL_SHADOWED::HI.val(hi_req),
                );
                self.registers.io_div2_meas_ctrl_shadowed.write(
                    IO_DIV2_MEAS_CTRL_SHADOWED::LO.val(lo_req)
                        + IO_DIV2_MEAS_CTRL_SHADOWED::HI.val(hi_req),
                );
            }
            MeasCtrlClk::IoDiv4 => {
                self.registers
                    .io_div4_meas_ctrl_en
                    .write(IO_DIV4_MEAS_CTRL_EN::EN.val(set_req));
                self.registers.io_div4_meas_ctrl_shadowed.write(
                    IO_DIV4_MEAS_CTRL_SHADOWED::LO.val(lo_req)
                        + IO_DIV4_MEAS_CTRL_SHADOWED::HI.val(hi_req),
                );
                self.registers.io_div4_meas_ctrl_shadowed.write(
                    IO_DIV4_MEAS_CTRL_SHADOWED::LO.val(lo_req)
                        + IO_DIV4_MEAS_CTRL_SHADOWED::HI.val(hi_req),
                );
            }
            MeasCtrlClk::MainClk => {
                self.registers
                    .main_meas_ctrl_en
                    .write(MAIN_MEAS_CTRL_EN::EN.val(set_req));
                self.registers.main_meas_ctrl_shadowed.write(
                    MAIN_MEAS_CTRL_SHADOWED::LO.val(lo_req)
                        + MAIN_MEAS_CTRL_SHADOWED::HI.val(hi_req),
                );
                self.registers.main_meas_ctrl_shadowed.write(
                    MAIN_MEAS_CTRL_SHADOWED::LO.val(lo_req)
                        + MAIN_MEAS_CTRL_SHADOWED::HI.val(hi_req),
                );
            }
            MeasCtrlClk::UsbClk => {
                self.registers
                    .usb_meas_ctrl_en
                    .write(USB_MEAS_CTRL_EN::EN.val(set_req));
                self.registers.usb_meas_ctrl_shadowed.write(
                    USB_MEAS_CTRL_SHADOWED::LO.val(lo_req) + USB_MEAS_CTRL_SHADOWED::HI.val(hi_req),
                );
                self.registers.usb_meas_ctrl_shadowed.write(
                    USB_MEAS_CTRL_SHADOWED::LO.val(lo_req) + USB_MEAS_CTRL_SHADOWED::HI.val(hi_req),
                );
            }
        };
        Ok(())
    }

    /// Lock the measure control setting. This is a sticky lock and can only be removed by reset.
    pub fn lock_meas_ctrl_setting(&self) {
        self.registers
            .measure_ctrl_regwen
            .write(MEASURE_CTRL_REGWEN::EN::CLEAR);
    }

    /// Get the the measure control lock setting. Returns 'true' if locked, false otherwise.
    pub fn is_locked_meas_ctrl_setting(&self) -> bool {
        match self
            .registers
            .measure_ctrl_regwen
            .read(MEASURE_CTRL_REGWEN::EN)
        {
            0x1 => false,
            _ => true,
        }
    }

    /// Gets the state of the speciffic clock measurement control.
    ///
    /// The 'clk' argument selects the clock it wants to query measurement control state for.
    ///
    /// Returns a tuple where the first 'bool' argument signifies if the clock measurement control
    /// is enabled, and the 2nd and 3rd members signify the lo and hi ranges respectively.
    pub fn get_clk_meas_ctrl_setting(&self, clk: MeasCtrlClk) -> (bool, u32, u32) {
        let en: u32;
        let lo: u32;
        let hi: u32;

        match clk {
            MeasCtrlClk::Io => {
                en = self.registers.io_meas_ctrl_en.read(IO_MEAS_CTRL_EN::EN);
                lo = self
                    .registers
                    .io_meas_ctrl_shadowed
                    .read(IO_MEAS_CTRL_SHADOWED::LO);
                hi = self
                    .registers
                    .io_meas_ctrl_shadowed
                    .read(IO_MEAS_CTRL_SHADOWED::HI);
            }
            MeasCtrlClk::IoDiv2 => {
                en = self
                    .registers
                    .io_div2_meas_ctrl_en
                    .read(IO_DIV2_MEAS_CTRL_EN::EN);
                lo = self
                    .registers
                    .io_div2_meas_ctrl_shadowed
                    .read(IO_DIV2_MEAS_CTRL_SHADOWED::LO);
                hi = self
                    .registers
                    .io_div2_meas_ctrl_shadowed
                    .read(IO_DIV2_MEAS_CTRL_SHADOWED::HI);
            }
            MeasCtrlClk::IoDiv4 => {
                en = self
                    .registers
                    .io_div4_meas_ctrl_en
                    .read(IO_DIV4_MEAS_CTRL_EN::EN);
                lo = self
                    .registers
                    .io_div4_meas_ctrl_shadowed
                    .read(IO_DIV4_MEAS_CTRL_SHADOWED::LO);
                hi = self
                    .registers
                    .io_div4_meas_ctrl_shadowed
                    .read(IO_DIV4_MEAS_CTRL_SHADOWED::HI);
            }
            MeasCtrlClk::MainClk => {
                en = self.registers.main_meas_ctrl_en.read(MAIN_MEAS_CTRL_EN::EN);
                lo = self
                    .registers
                    .main_meas_ctrl_shadowed
                    .read(MAIN_MEAS_CTRL_SHADOWED::LO);
                hi = self
                    .registers
                    .main_meas_ctrl_shadowed
                    .read(MAIN_MEAS_CTRL_SHADOWED::HI);
            }
            MeasCtrlClk::UsbClk => {
                en = self.registers.usb_meas_ctrl_en.read(USB_MEAS_CTRL_EN::EN);
                lo = self
                    .registers
                    .usb_meas_ctrl_shadowed
                    .read(USB_MEAS_CTRL_SHADOWED::LO);
                hi = self
                    .registers
                    .usb_meas_ctrl_shadowed
                    .read(USB_MEAS_CTRL_SHADOWED::HI);
            }
        };
        let en_sts = matches!(en, MULTI_BIT_BOOL_4TRUE);
        (en_sts, lo, hi)
    }

    /// Queries if a specific error code is present.
    ///
    /// The 'err' argument selects the error it wants to query for.
    ///
    /// Returns 'true' if the error is present and 'false' otherwise
    pub fn is_recov_err_code_present(&self, err: Option<RecovErr>) -> bool {
        match err {
            Some(RecovErr::UsbTimeoutErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::USB_TIMEOUT_ERR),
            Some(RecovErr::MainTimeoutErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::MAIN_TIMEOUT_ERR),
            Some(RecovErr::IoDiv4TimeoutErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::IO_DIV4_TIMEOUT_ERR),
            Some(RecovErr::IoDiv2TimeoutErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::IO_DIV2_TIMEOUT_ERR),
            Some(RecovErr::IoTimeoutErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::IO_TIMEOUT_ERR),
            Some(RecovErr::UsbMeasureErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::USB_MEASURE_ERR),
            Some(RecovErr::MainMeasureErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::MAIN_MEASURE_ERR),
            Some(RecovErr::IoDiv4MeasureErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::IO_DIV4_MEASURE_ERR),
            Some(RecovErr::IoDiv2MeasureErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::IO_DIV2_MEASURE_ERR),
            Some(RecovErr::IoMeasureErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::IO_MEASURE_ERR),
            Some(RecovErr::ShadowUpdateErr) => self
                .registers
                .recov_err_code
                .is_set(RECOV_ERR_CODE::SHADOW_UPDATE_ERR),
            None => self.registers.recov_err_code.get() == 0x0,
        }
    }

    /// Clears a specific error code.
    ///
    /// The 'err' argument selects the error it wants to clear.
    pub fn clear_recov_err_code(&self, err: RecovErr) {
        match err {
            RecovErr::UsbTimeoutErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::USB_TIMEOUT_ERR::SET),
            RecovErr::MainTimeoutErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::MAIN_TIMEOUT_ERR::SET),
            RecovErr::IoDiv4TimeoutErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::IO_DIV4_TIMEOUT_ERR::SET),
            RecovErr::IoDiv2TimeoutErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::IO_DIV2_TIMEOUT_ERR::SET),
            RecovErr::IoTimeoutErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::IO_TIMEOUT_ERR::SET),
            RecovErr::MainMeasureErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::MAIN_MEASURE_ERR::SET),
            RecovErr::UsbMeasureErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::USB_MEASURE_ERR::SET),
            RecovErr::IoDiv4MeasureErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::IO_DIV4_MEASURE_ERR::SET),
            RecovErr::IoDiv2MeasureErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::IO_DIV2_MEASURE_ERR::SET),
            RecovErr::IoMeasureErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::IO_MEASURE_ERR::SET),
            RecovErr::ShadowUpdateErr => self
                .registers
                .recov_err_code
                .write(RECOV_ERR_CODE::SHADOW_UPDATE_ERR::SET),
        };
    }

    /// Queries if a specific fatal error code is present.
    ///
    /// The 'err' argument selects the fatal error it wants to query for.
    ///
    /// Returns 'true' if the  fatal error is present and 'false' otherwise
    pub fn is_fatal_err_code_present(&self, err: Option<FatalErr>) -> bool {
        match err {
            Some(FatalErr::ShadowStorageErr) => self
                .registers
                .fatal_err_code
                .is_set(FATAL_ERR_CODE::SHADOW_STORAGE_ERR),
            Some(FatalErr::IdleCnt) => self
                .registers
                .fatal_err_code
                .is_set(FATAL_ERR_CODE::IDLE_CNT),
            Some(FatalErr::RegIntg) => self
                .registers
                .fatal_err_code
                .is_set(FATAL_ERR_CODE::REG_INTG),
            None => self.registers.fatal_err_code.get() == 0x0,
        }
    }
    #[cfg(feature = "clkmgr_tests")]
    /// Test runner that contains a set of unit tests to run on target for the clkmgr.
    pub fn run_tests(&self) -> bool {
        debug!("* Start running clkmgr tests!");

        // Run tests on extclk interactions
        test_helper("Check exclk On High Speed timeout 0 ", || {
            self.set_extclk(ExtClkState::ExtClkOnHighSpeed, 0) == Err(ErrorCode::BUSY)
        });
        test_helper("Check exclk On High Speed timeout 0 ", || {
            self.set_extclk(ExtClkState::ExtClkError, 0) == Err(ErrorCode::INVAL)
        });
        test_helper("Check exclk On High Speed timeout 1000 ", || {
            (self.set_extclk(ExtClkState::ExtClkOnHighSpeed, 10000) == Ok(()))
                && (self.registers.extclk_ctrl.get() == 0x66)
                && (self.registers.extclk_status.get() == 0x6)
        });
        test_helper("Check exclk status ExtClkOnHighSpeed ", || {
            self.get_extclk_state() == ExtClkState::ExtClkOnHighSpeed
        });
        test_helper("Check exclk On ExtClkOnLowSpeed ", || {
            (self.set_extclk(ExtClkState::ExtClkOnLowSpeed, 1000) == Ok(()))
                && (self.registers.extclk_ctrl.get() == 0x96)
                && (self.registers.extclk_status.get() == 0x6)
        });
        test_helper("Check exclk status ExtClkOnLowSpeed ", || {
            (self.get_extclk_state() == ExtClkState::ExtClkOnLowSpeed)
                && (self.registers.extclk_ctrl.get() == 0x96)
                && (self.registers.extclk_status.get() == 0x6)
        });
        test_helper("Check exclk On ExtClkOff ", || {
            (self.set_extclk(ExtClkState::ExtClkOff, 10000) == Ok(()))
                && (self.registers.extclk_ctrl.get() == 0x99)
                && (self.registers.extclk_status.get() == 0x9)
        });
        test_helper("Check exclk status ExtClkOff ", || {
            (self.get_extclk_state() == ExtClkState::ExtClkOff)
                && (self.registers.extclk_ctrl.get() == 0x99)
                && (self.registers.extclk_status.get() == 0x9)
        });

        // Run tests on clock jitter
        self.set_clk_jitter(true);
        test_helper("Check Jitter enable ", || {
            self.is_clk_jitter_enabled() && (self.registers.jitter_enable.get() == 0x6)
        });

        self.set_clk_jitter(false);
        test_helper("Check Jitter disable ", || {
            !self.is_clk_jitter_enabled() && (self.registers.jitter_enable.get() == 0x9)
        });

        self.set_clk_jitter(true);
        test_helper("Check Jitter enable ", || {
            self.is_clk_jitter_enabled() && (self.registers.jitter_enable.get() == 0x6)
        });

        // Define tests for gateble clocks as an array
        let cklist = [
            GateableClk::UsbPeri,
            GateableClk::IoPeri,
            GateableClk::IoDiv2Peri,
            GateableClk::IoDiv4Peri,
            GateableClk::TransClkMainOTBN,
            GateableClk::TransClkMainKMAC,
            GateableClk::TransClkMainHMAC,
            GateableClk::TransClkMainAES,
        ];
        // Run the tests for all clocks.
        for iter in cklist.iter() {
            self.set_clk_enable(*iter, false);
            test_helper(" Check Clk disable ", || !self.is_clk_enabled(*iter));
            self.set_clk_enable(*iter, true);
            test_helper(" Check Clk enable ", || self.is_clk_enabled(*iter));
        }

        // Test the recoverable error reading before messing with the measurement control because that will induce errors.
        // We use the same trick with an array of tuples.
        let cklist = [
            (RecovErr::UsbTimeoutErr, false, 0x0),
            (RecovErr::MainTimeoutErr, false, 0x0),
            (RecovErr::IoDiv4TimeoutErr, false, 0x0),
            (RecovErr::IoDiv2TimeoutErr, false, 0x0),
            (RecovErr::IoTimeoutErr, false, 0x0),
            (RecovErr::UsbMeasureErr, false, 0x0),
            (RecovErr::IoDiv4MeasureErr, false, 0x0),
            (RecovErr::IoDiv2MeasureErr, false, 0x0),
            (RecovErr::IoMeasureErr, false, 0x0),
            (RecovErr::ShadowUpdateErr, false, 0x0),
        ];
        for (err, expected_ret, expected_reg) in cklist {
            test_helper(" Check Recoverable error reading ", || {
                self.is_recov_err_code_present(Some(err)) == expected_ret
                    && (self.registers.recov_err_code.get() == expected_reg)
            });
        }

        // Test all interactions for setting measurement controls and see the error cases as well.
        let cklist = [
            (MeasCtrlClk::Io, true, 0x2, 0xA, Ok(())),
            (MeasCtrlClk::Io, true, 0xA, 0xB, Ok(())),
            (MeasCtrlClk::Io, true, 0x4, 0xC, Ok(())),
            (MeasCtrlClk::Io, true, 0xA, 0xFFFF, Err(ErrorCode::INVAL)),
            (MeasCtrlClk::Io, false, 0x1D6, 0x1EA, Err(ErrorCode::INVAL)),
            (MeasCtrlClk::IoDiv2, true, 0, 10, Ok(())),
            (MeasCtrlClk::IoDiv2, true, 0, 0xFFFF, Err(ErrorCode::INVAL)),
            (MeasCtrlClk::IoDiv2, false, 0xE6, 0xFA, Ok(())),
            (MeasCtrlClk::IoDiv4, true, 0, 10, Ok(())),
            (MeasCtrlClk::IoDiv4, true, 0, 0xFFFF, Err(ErrorCode::INVAL)),
            (MeasCtrlClk::IoDiv4, false, 0x6E, 0x82, Ok(())),
            (MeasCtrlClk::MainClk, true, 0, 10, Ok(())),
            (MeasCtrlClk::MainClk, true, 0, 0xFFFF, Err(ErrorCode::INVAL)),
            (MeasCtrlClk::MainClk, false, 0x1FA, 0x1FE, Ok(())),
            (MeasCtrlClk::UsbClk, true, 0, 10, Ok(())),
            (MeasCtrlClk::UsbClk, true, 0, 0xFFFF, Err(ErrorCode::INVAL)),
            (MeasCtrlClk::UsbClk, false, 0xE6, 0xFA, Ok(())),
        ];

        for (clock, clk_meas_set, lo, hi, expected_response) in cklist {
            let current_resp = self.set_clk_meas_ctrl(clock, clk_meas_set, lo, hi);
            if Ok(()) == expected_response {
                test_helper(" Check Clk Meas set ", || {
                    self.get_clk_meas_ctrl_setting(clock) == (clk_meas_set, lo, hi)
                        && (expected_response == current_resp)
                });
            }
        }

        // Check getting recoverable errors and confront with the actual register value.
        // This must run _after_ clk_meas_ctrl setting since that will actually induce the clocke errors.
        let cklist = [
            (RecovErr::UsbTimeoutErr, false, 0x3E),
            (RecovErr::MainTimeoutErr, false, 0x3E),
            (RecovErr::IoDiv4TimeoutErr, false, 0x3E),
            (RecovErr::IoDiv2TimeoutErr, false, 0x3E),
            (RecovErr::IoTimeoutErr, false, 0x3E),
            (RecovErr::UsbMeasureErr, true, 0x3E),
            (RecovErr::MainMeasureErr, true, 0x1E),
            (RecovErr::IoDiv4MeasureErr, true, 0xE),
            (RecovErr::IoDiv2MeasureErr, true, 0x6),
            (RecovErr::IoMeasureErr, true, 0x2),
            (RecovErr::ShadowUpdateErr, false, 0x0),
        ];
        for (err, expected_ret, expected_reg) in cklist {
            test_helper(" Check Recoverable error reading and clearing", || {
                self.is_recov_err_code_present(Some(err)) == expected_ret
                    && (self.registers.recov_err_code.get() == expected_reg)
            });
            if expected_ret {
                self.clear_recov_err_code(err);
                test_helper(" Check Recoverable error clearing", || {
                    !self.is_recov_err_code_present(Some(err))
                });
            }
        }

        // Check reading fatal errors, those I just check the absence of because we don't have a readily
        // available way of stimulation.
        let cklist = [
            (FatalErr::ShadowStorageErr, false, 0x0),
            (FatalErr::IdleCnt, false, 0x0),
            (FatalErr::RegIntg, false, 0x0),
        ];
        for (err, expected_ret, expected_reg) in cklist {
            test_helper(" Check Non-recoverable error reading and clearing", || {
                self.is_fatal_err_code_present(Some(err)) == expected_ret
                    && (self.registers.fatal_err_code.get() == expected_reg)
            });
        }
        debug!("* Finished running clkmgr tests!");

        true
    }
}
