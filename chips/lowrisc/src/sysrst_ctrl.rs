// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::registers::{
    rv_timer_regs::INTR_ENABLE0,
    sysrst_ctrl_regs::{
        SysrstCtrlRegisters, AUTO_BLOCK_DEBOUNCE_CTL, AUTO_BLOCK_OUT_CTL, COM_DET_CTL, COM_OUT_CTL,
        COM_PRE_DET_CTL, COM_PRE_SEL_CTL, COM_SEL_CTL, EC_RST_CTL, INTR, KEY_INTR_CTL,
        KEY_INTR_DEBOUNCE_CTL, KEY_INVERT_CTL, PIN_ALLOWED_CTL, PIN_IN_VALUE, PIN_OUT_CTL, REGWEN,
        ULP_AC_DEBOUNCE_CTL, ULP_CTL, ULP_LID_DEBOUNCE_CTL, ULP_PWRB_DEBOUNCE_CTL, ULP_STATUS,
    },
};
use kernel::utilities::{
    registers::interfaces::{ReadWriteable, Readable, Writeable},
    StaticRef,
};

//TODO: how to import this base address from top_earlgrey? if this file is moved to chips/earlgrey than SysrstCtrlRegisters memebers are not accessible
pub const SYSRST_CTRL_AON_BASE_ADDR: usize = 0x40430000;

pub const SYSRST_CTRL_AON_BASE: StaticRef<SysrstCtrlRegisters> =
    unsafe { StaticRef::new(SYSRST_CTRL_AON_BASE_ADDR as *const SysrstCtrlRegisters) };

pub struct SysRstCtrlComboDetectorPins {
    ac_present: bool,
    pwrb: bool,
    key0: bool,
    key1: bool,
    key2: bool,
}

pub struct SysRstCtrlComboDetectorAction {
    rst_req: bool,
    ec_rst: bool,
    interrupt: bool,
    bat_disable: bool,
}

pub struct SRCKeyInterruptConfig {
    pub debounce_timer_us: u16,
    pub pwrb_H2L: bool,
    pub pwrb_L2H: bool,
    pub key0_H2L: bool,
    pub key0_L2H: bool,
    pub key1_H2L: bool,
    pub key1_L2H: bool,
    pub key2_H2L: bool,
    pub key2_L2H: bool,
    pub ac_present_H2L: bool,
    pub ac_present_L2H: bool,
    pub ec_reset_H2L: bool,
    pub ec_reset_L2H: bool,
    pub flash_wp_H2L: bool,
    pub flash_wp_L2H: bool,
}

pub enum SRCInputPin {
    Pwrb,
    Key0,
    Key1,
    Key2,
    LidOpen,
    AcPresent,
    EcReset,
    FlashWP,
}

pub enum SRCOutputPin {
    Pwrb,
    BatDisable,
    EcReset,
    Key0,
    Key1,
    Key2,
    Z3Wakeup,
    FlashWP,
}

pub struct SRCAllowedPinConfig {
    bat_disable_0: bool,
    bat_disable_1: bool,
    ec_reset_0: bool,
    ec_reset_1: bool,
    pwrb_0: bool,
    pwrb_1: bool,
    key0_0: bool,
    key0_1: bool,
    key1_0: bool,
    key1_1: bool,
    key2_0: bool,
    key2_1: bool,
    z3_wakeup_0: bool,
    z3_wakeup_1: bool,
    flash_wp_0: bool,
    flash_wp_1: bool,
}

pub struct SRCWakeupConfig {
    pub ac_present_debounce_timer_us: u16,
    pub pwrb_debounce_timer_us: u16,
    pub lid_open_debounce_timer_us: u16,
}

pub struct SysRstCtrlComboDetectorConfiguration {
    precondition: SysRstCtrlComboDetectorPins,
    precondition_time_us: u32,
    condition: SysRstCtrlComboDetectorPins,
    condition_time_us: u32,
    action: SysRstCtrlComboDetectorAction,
}

pub struct SysRstCtrlAutoblock {
    pub pwrb_debounce_timer_us: u32,
    pub block_key0: Option<bool>,
    pub block_key1: Option<bool>,
    pub block_key2: Option<bool>,
}
pub struct SysRstCtrl {
    registers: StaticRef<SysrstCtrlRegisters>,
}

impl SysRstCtrl {
    pub fn new() -> Self {
        Self {
            registers: SYSRST_CTRL_AON_BASE,
        }
    }

    pub fn is_configuration_locked(&self) -> bool {
        !self.registers.regwen.is_set(REGWEN::WRITE_EN)
    }

    pub fn lock_configuration(&self) {
        self.registers.regwen.write(REGWEN::WRITE_EN.val(0));
    }

    /* COMBO DETECTOR */
    /// configure combo detector functionality
    /// conf._timer_ns will be divided by 5 as register content is in mutliple of 5us
    pub fn configure_combo_detector(
        &self,
        detector_id: u8,
        configuration: &SysRstCtrlComboDetectorConfiguration,
    ) -> Result<(), ()> {
        if detector_id >= 4 {
            return Err(());
        }

        if self.is_configuration_locked() {
            return Err(());
        }

        // configure action
        self.registers.com_out_ctl[detector_id as usize].write(
            COM_OUT_CTL::BAT_DISABLE_0.val(configuration.action.bat_disable as u32)
                + COM_OUT_CTL::INTERRUPT_0.val(configuration.action.interrupt as u32)
                + COM_OUT_CTL::EC_RST_0.val(configuration.action.ec_rst as u32)
                + COM_OUT_CTL::RST_REQ_0.val(configuration.action.rst_req as u32),
        );

        //configure condition using COMP_SEL_CTL
        self.registers.com_sel_ctl[detector_id as usize].write(
            COM_SEL_CTL::KEY0_IN_SEL_0.val(configuration.condition.key0 as u32)
                + COM_SEL_CTL::KEY1_IN_SEL_0.val(configuration.condition.key1 as u32)
                + COM_SEL_CTL::KEY2_IN_SEL_0.val(configuration.condition.key2 as u32)
                + COM_SEL_CTL::AC_PRESENT_SEL_0.val(configuration.condition.ac_present as u32)
                + COM_SEL_CTL::PWRB_IN_SEL_0.val(configuration.condition.pwrb as u32),
        );

        // configure main condition detection timer using COMP_DET_CTL
        self.registers.com_det_ctl[detector_id as usize].write(
            COM_DET_CTL::DETECTION_TIMER_0.val(u32::div_ceil(configuration.condition_time_us, 5)),
        );

        // configure precondition using COM_PRE_SEL_CTL
        self.registers.com_pre_sel_ctl[detector_id as usize].write(
            COM_PRE_SEL_CTL::KEY0_IN_SEL_0.val(configuration.precondition.key0 as u32)
                + COM_PRE_SEL_CTL::KEY1_IN_SEL_0.val(configuration.precondition.key1 as u32)
                + COM_PRE_SEL_CTL::KEY2_IN_SEL_0.val(configuration.precondition.key2 as u32)
                + COM_PRE_SEL_CTL::AC_PRESENT_SEL_0
                    .val(configuration.precondition.ac_present as u32)
                + COM_PRE_SEL_CTL::PWRB_IN_SEL_0.val(configuration.precondition.pwrb as u32),
        );

        // configure precondition detection timer using COM_PRE_DET_CTL
        self.registers.com_pre_det_ctl[detector_id as usize].write(
            COM_PRE_DET_CTL::PRECONDITION_TIMER_0
                .val(u32::div_ceil(configuration.precondition_time_us, 5)),
        );

        Ok(())
    }

    pub fn configure_pulselength_ec_rst_l_o(&self, duration_us: u32) {
        self.registers
            .ec_rst_ctl
            .write(EC_RST_CTL::EC_RST_PULSE.val(u32::div_ceil(duration_us, 5)));
    }

    pub fn configure_autoblock(&self, configuration: SysRstCtrlAutoblock) {
        let key0_out_sel = configuration.block_key0.is_some() as u32;
        let key0_out_value = configuration.block_key0.unwrap_or(false) as u32;
        let key1_out_sel = configuration.block_key1.is_some() as u32;
        let key1_out_value = configuration.block_key1.unwrap_or(false) as u32;
        let key2_out_sel = configuration.block_key2.is_some() as u32;
        let key2_out_value = configuration.block_key2.unwrap_or(false) as u32;

        self.registers.auto_block_out_ctl.write(
            AUTO_BLOCK_OUT_CTL::KEY2_OUT_VALUE.val(key2_out_value)
                + AUTO_BLOCK_OUT_CTL::KEY2_OUT_SEL.val(key2_out_sel)
                + AUTO_BLOCK_OUT_CTL::KEY1_OUT_VALUE.val(key1_out_value)
                + AUTO_BLOCK_OUT_CTL::KEY1_OUT_SEL.val(key1_out_sel)
                + AUTO_BLOCK_OUT_CTL::KEY0_OUT_VALUE.val(key0_out_value)
                + AUTO_BLOCK_OUT_CTL::KEY0_OUT_SEL.val(key0_out_sel),
        );

        // enable autoblock and configure debounce time
        self.registers.auto_block_debounce_ctl.write(
            AUTO_BLOCK_DEBOUNCE_CTL::DEBOUNCE_TIMER
                .val(u32::div_ceil(configuration.pwrb_debounce_timer_us, 5))
                + AUTO_BLOCK_DEBOUNCE_CTL::AUTO_BLOCK_ENABLE.val(1),
        );
    }

    pub fn configure_keyinterrupt(&self, configuration: SRCKeyInterruptConfig) {
        self.registers.key_intr_debounce_ctl.write(
            KEY_INTR_DEBOUNCE_CTL::DEBOUNCE_TIMER
                .val(u16::div_ceil(configuration.debounce_timer_us, 5) as u32),
        );

        self.registers.key_intr_ctl.write(
            KEY_INTR_CTL::FLASH_WP_L_H2L.val(configuration.flash_wp_H2L as u32)
                + KEY_INTR_CTL::FLASH_WP_L_L2H.val(configuration.flash_wp_L2H as u32)
                + KEY_INTR_CTL::EC_RST_L_H2L.val(configuration.ec_reset_H2L as u32)
                + KEY_INTR_CTL::EC_RST_L_L2H.val(configuration.ec_reset_L2H as u32)
                + KEY_INTR_CTL::AC_PRESENT_H2L.val(configuration.ac_present_H2L as u32)
                + KEY_INTR_CTL::AC_PRESENT_L2H.val(configuration.ac_present_L2H as u32)
                + KEY_INTR_CTL::KEY2_IN_H2L.val(configuration.key2_H2L as u32)
                + KEY_INTR_CTL::KEY2_IN_L2H.val(configuration.key2_L2H as u32)
                + KEY_INTR_CTL::KEY1_IN_H2L.val(configuration.key1_H2L as u32)
                + KEY_INTR_CTL::KEY1_IN_L2H.val(configuration.key1_L2H as u32)
                + KEY_INTR_CTL::KEY0_IN_H2L.val(configuration.key0_H2L as u32)
                + KEY_INTR_CTL::KEY0_IN_L2H.val(configuration.key0_L2H as u32)
                + KEY_INTR_CTL::PWRB_IN_H2L.val(configuration.pwrb_H2L as u32)
                + KEY_INTR_CTL::PWRB_IN_L2H.val(configuration.pwrb_L2H as u32),
        );
    }

    pub fn configure_wakeup(&self, configuration: SRCWakeupConfig) {
        self.registers.ulp_ac_debounce_ctl.write(
            ULP_AC_DEBOUNCE_CTL::ULP_AC_DEBOUNCE_TIMER
                .val(u16::div_ceil(configuration.ac_present_debounce_timer_us, 5) as u32),
        );
        self.registers.ulp_lid_debounce_ctl.write(
            ULP_LID_DEBOUNCE_CTL::ULP_LID_DEBOUNCE_TIMER
                .val(u16::div_ceil(configuration.lid_open_debounce_timer_us, 5) as u32),
        );
        self.registers.ulp_pwrb_debounce_ctl.write(
            ULP_PWRB_DEBOUNCE_CTL::ULP_PWRB_DEBOUNCE_TIMER
                .val(u16::div_ceil(configuration.pwrb_debounce_timer_us, 5) as u32),
        );

        // enable ULP Wakeup
        self.registers.ulp_ctl.write(ULP_CTL::ULP_ENABLE.val(1));
    }

    //TODO: ULP_STATUS or WKUP_STATUS
    pub fn wakeup_detected(&self) -> bool {
        self.registers.ulp_status.read(ULP_STATUS::ULP_WAKEUP) != 0
    }

    /// ULP Wakeup registers are on the 200kHz AON clock domain. In order to reset the wakeup detection capability, the code needs to disable WKUP, wait for the ULP_ENABLE bit to become 0 and then enable WKUP. This will take some time (~200kHz), waiting in a busy loop would take too much time from other tasks, a deffred task would leave a significant gap when WKKUP detection is disabled. The chosen solution is to split this operation in 2 functions and inside the interrupt handle to start the process, execute other functions and finally finish this process by busy waiting, hoping that the process can be finished now
    pub fn clear_wakeup(&self) {
        // wait for ULP_ENABLE to be cleared
        while self.registers.ulp_ctl.is_set(ULP_CTL::ULP_ENABLE) {}
        // enable ULP_ENABLE
        self.registers.ulp_ctl.write(ULP_CTL::ULP_ENABLE.val(1));
    }

    pub fn preclear_wakeup(&self) {
        self.registers.ulp_ctl.write(ULP_CTL::ULP_ENABLE.val(0));
    }

    pub fn enable_interrupts(&self) {
        self.registers
            .intr_enable
            .write(INTR::EVENT_DETECTED.val(1));
    }

    /* PIN INPUT VALUE */
    pub fn get_input_pin_state(&self, pin: &SRCInputPin) -> bool {
        let state = self.registers.pin_in_value.extract();
        match pin {
            SRCInputPin::Pwrb => state.is_set(PIN_IN_VALUE::PWRB_IN),
            SRCInputPin::Key0 => state.is_set(PIN_IN_VALUE::KEY0_IN),
            SRCInputPin::Key1 => state.is_set(PIN_IN_VALUE::KEY1_IN),
            SRCInputPin::Key2 => state.is_set(PIN_IN_VALUE::KEY2_IN),
            SRCInputPin::LidOpen => state.is_set(PIN_IN_VALUE::LID_OPEN),
            SRCInputPin::AcPresent => state.is_set(PIN_IN_VALUE::AC_PRESENT),
            SRCInputPin::EcReset => state.is_set(PIN_IN_VALUE::EC_RST_L),
            SRCInputPin::FlashWP => state.is_set(PIN_IN_VALUE::FLASH_WP_L),
        }
    }

    pub fn get_all_input_pins_state(&self) -> u32 {
        //TODO: convert u32 to u8 another way
        self.registers.pin_in_value.get()
    }

    /* PIN OUTPUT VALUE */

    pub fn get_all_output_pins_state(&self) -> u32 {
        //TODO: convert u32 to u8 another way
        self.registers.pin_out_value.get()
    }

    pub fn modify_output_pin_state(&self, pin: SRCOutputPin, state: bool) {
        let register = &self.registers.pin_out_ctl;
        match pin {
            SRCOutputPin::Pwrb => register.modify(PIN_OUT_CTL::PWRB_OUT.val(state as u32)),
            SRCOutputPin::BatDisable => register.modify(PIN_OUT_CTL::BAT_DISABLE.val(state as u32)),
            SRCOutputPin::EcReset => register.modify(PIN_OUT_CTL::EC_RST_L.val(state as u32)),
            SRCOutputPin::Key0 => register.modify(PIN_OUT_CTL::KEY0_OUT.val(state as u32)),
            SRCOutputPin::Key1 => register.modify(PIN_OUT_CTL::KEY1_OUT.val(state as u32)),
            SRCOutputPin::Key2 => register.modify(PIN_OUT_CTL::KEY2_OUT.val(state as u32)),
            SRCOutputPin::Z3Wakeup => register.modify(PIN_OUT_CTL::Z3_WAKEUP.val(state as u32)),
            SRCOutputPin::FlashWP => register.modify(PIN_OUT_CTL::FLASH_WP_L.val(state as u32)),
        }
    }

    pub fn configure_pin_inverter(&self) {
        self.registers
            .key_invert_ctl
            .write(KEY_INVERT_CTL::KEY0_OUT.val(1) + KEY_INVERT_CTL::PWRB_OUT.val(1));
    }

    pub fn key_interrupt_status(&self) -> u32 {
        self.registers.key_intr_status.get()
    }

    pub fn configured_allowed_pin_states(&self, config: &SRCAllowedPinConfig) {
        self.registers.pin_allowed_ctl.write(
            PIN_ALLOWED_CTL::BAT_DISABLE_0.val(config.bat_disable_0 as u32)
                + PIN_ALLOWED_CTL::BAT_DISABLE_1.val(config.bat_disable_1 as u32)
                + PIN_ALLOWED_CTL::EC_RST_L_0.val(config.ec_reset_0 as u32)
                + PIN_ALLOWED_CTL::EC_RST_L_1.val(config.ec_reset_1 as u32)
                + PIN_ALLOWED_CTL::PWRB_OUT_0.val(config.pwrb_0 as u32)
                + PIN_ALLOWED_CTL::PWRB_OUT_1.val(config.pwrb_1 as u32)
                + PIN_ALLOWED_CTL::KEY0_OUT_0.val(config.key0_0 as u32)
                + PIN_ALLOWED_CTL::KEY0_OUT_1.val(config.key1_1 as u32)
                + PIN_ALLOWED_CTL::KEY1_OUT_0.val(config.key1_0 as u32)
                + PIN_ALLOWED_CTL::KEY1_OUT_1.val(config.key2_1 as u32)
                + PIN_ALLOWED_CTL::KEY2_OUT_0.val(config.key2_0 as u32)
                + PIN_ALLOWED_CTL::KEY2_OUT_1.val(config.key2_1 as u32)
                + PIN_ALLOWED_CTL::Z3_WAKEUP_0.val(config.z3_wakeup_0 as u32)
                + PIN_ALLOWED_CTL::Z3_WAKEUP_1.val(config.z3_wakeup_1 as u32)
                + PIN_ALLOWED_CTL::FLASH_WP_L_0.val(config.flash_wp_0 as u32)
                + PIN_ALLOWED_CTL::FLASH_WP_L_1.val(config.flash_wp_1 as u32),
        );
    }
}
