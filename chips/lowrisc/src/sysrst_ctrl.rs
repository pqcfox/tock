// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header has to be included to be able to submit it to Tock
// It is up to ZeroRISC to decide if it keeps this header or not

use crate::registers::sysrst_ctrl_regs::{
    SysrstCtrlRegisters, AUTO_BLOCK_DEBOUNCE_CTL, AUTO_BLOCK_OUT_CTL, COMBO_INTR_STATUS,
    COM_DET_CTL, COM_OUT_CTL, COM_PRE_DET_CTL, COM_PRE_SEL_CTL, COM_SEL_CTL, EC_RST_CTL, INTR,
    KEY_INTR_CTL, KEY_INTR_DEBOUNCE_CTL, KEY_INTR_STATUS, KEY_INVERT_CTL, PIN_ALLOWED_CTL,
    PIN_IN_VALUE, PIN_OUT_CTL, PIN_OUT_VALUE, REGWEN, ULP_AC_DEBOUNCE_CTL, ULP_CTL,
    ULP_LID_DEBOUNCE_CTL, ULP_PWRB_DEBOUNCE_CTL, ULP_STATUS, WKUP_STATUS,
};
use kernel::{
    hil::opentitan_sysrst::{
        OpenTitanSysRstr, OpenTitanSysRstrClient, SRCAllowedPinConfig, SRCAutoblockConfig,
        SRCComboDetectorAction, SRCComboDetectorConfig, SRCComboDetectorId, SRCComboDetectorPins,
        SRCInputPin, SRCInputPinState, SRCInputPinStatus, SRCKeyInterruptConfig, SRCOutputPin,
        SRCPinInversionConfig, SRCWakeupConfig,
    },
    utilities::{
        cells::OptionalCell,
        registers::{
            interfaces::{ReadWriteable, Readable, Writeable},
            Field, InMemoryRegister, LocalRegisterCopy,
        },
        StaticRef,
    },
};

pub struct SysRstCtrl<'a> {
    registers: StaticRef<SysrstCtrlRegisters>,
    client: OptionalCell<&'a dyn OpenTitanSysRstrClient>,
}

#[derive(Clone, Copy)]
pub enum SysRstCtrlInterrupt {
    AonEventDetected,
}

impl<'a> SysRstCtrl<'a> {
    pub fn new(register_base: usize) -> Self {
        Self {
            registers: unsafe { StaticRef::new(register_base as *const SysrstCtrlRegisters) },
            client: OptionalCell::empty(),
        }
    }

    pub fn set_client(&self, client: Option<&'a dyn OpenTitanSysRstrClient>) {
        self.client.insert(client);
    }

    pub fn is_configuration_locked(&self) -> bool {
        !self.registers.regwen.is_set(REGWEN::WRITE_EN)
    }

    /* COMBO DETECTOR */

    pub fn is_combo_detector_interrupt_triggered(&self, detector_id: SRCComboDetectorId) -> bool {
        let field = match detector_id {
            SRCComboDetectorId::Zero => COMBO_INTR_STATUS::COMBO0_H2L,
            SRCComboDetectorId::One => COMBO_INTR_STATUS::COMBO1_H2L,
            SRCComboDetectorId::Two => COMBO_INTR_STATUS::COMBO2_H2L,
            SRCComboDetectorId::Three => COMBO_INTR_STATUS::COMBO3_H2L,
        };
        self.registers.combo_intr_status.is_set(field)
    }

    pub fn clear_combo_detector_interrupt_status(&self, detector_id: SRCComboDetectorId) {
        let field = match detector_id {
            SRCComboDetectorId::Zero => COMBO_INTR_STATUS::COMBO0_H2L,
            SRCComboDetectorId::One => COMBO_INTR_STATUS::COMBO1_H2L,
            SRCComboDetectorId::Two => COMBO_INTR_STATUS::COMBO2_H2L,
            SRCComboDetectorId::Three => COMBO_INTR_STATUS::COMBO3_H2L,
        };
        self.registers.combo_intr_status.write(field.val(1));
    }
    pub fn configure_pulselength_ec_rst_l_o(&self, duration_us: u32) {
        self.registers
            .ec_rst_ctl
            .write(EC_RST_CTL::EC_RST_PULSE.val(u32::div_ceil(duration_us, 5)));
    }

    pub fn get_pulselength_ec_rst_l_o_duration(&self) -> u32 {
        self.registers.ec_rst_ctl.read(EC_RST_CTL::EC_RST_PULSE) * 5
    }

    /* AUTO BLOCK */
    /// helper function for reading autoblock configuration
    fn autoblock_option_from_bits(out_sel: bool, out_value: bool) -> Option<bool> {
        match (out_sel, out_value) {
            (true, x) => Some(x),
            (false, _) => None,
        }
    }

    /* KEY INTERRUPT */
    pub fn key_interrupt_status(&self) -> LocalRegisterCopy<u32, KEY_INTR_STATUS::Register> {
        let value = self.registers.key_intr_status.get();
        LocalRegisterCopy::<u32, KEY_INTR_STATUS::Register>::new(value)
    }

    pub fn key_interrupt_clear(&self, a: Field<u32, KEY_INTR_STATUS::Register>) {
        let b = Field::<u32, KEY_INTR_STATUS::Register>::new(a.mask, a.shift);
        self.registers.key_intr_status.modify(b.val(1));
    }

    /* WAKEUP */

    /// WKUP_STATUS can be triggered by ULP_WAKEUP circuit and by Key Interrupt
    pub fn wakeup_detected(&self) -> bool {
        self.registers.wkup_status.is_set(WKUP_STATUS::WAKEUP_STS)
    }

    pub fn clear_wakeup(&self) {
        self.registers
            .wkup_status
            .write(WKUP_STATUS::WAKEUP_STS.val(1));
    }

    /* ULP WAKEUP */

    pub fn ulp_wakeup_detected(&self) -> bool {
        self.registers.ulp_status.is_set(ULP_STATUS::ULP_WAKEUP)
    }

    /// ULP Wakeup registers are on the 200kHz AON clock domain. In order to reset the wakeup detection capability, the code needs to disable WKUP, wait for the ULP_ENABLE bit to become 0 and then enable WKUP. This will take some time (~200kHz), waiting in a busy loop would take too much time from other tasks, a deffred task would leave a significant gap when WKKUP detection is disabled. The chosen solution is to split this operation in 2 functions and inside the interrupt handle to start the process, execute other functions and finally finish this process by busy waiting, hoping that the process can be finished now
    pub fn reset_ulp_wakeup(&self) {
        // wait for ULP_ENABLE to be cleared
        while self.registers.ulp_ctl.is_set(ULP_CTL::ULP_ENABLE) {}
        // enable ULP_ENABLE
        self.registers.ulp_ctl.write(ULP_CTL::ULP_ENABLE.val(1));
    }

    pub fn clear_ulp_wakeup(&self) {
        self.registers.ulp_ctl.write(ULP_CTL::ULP_ENABLE.val(0));
    }

    /* INTERRUPTS */
    pub fn enable_interrupts(&self) {
        self.registers
            .intr_enable
            .write(INTR::EVENT_DETECTED.val(1));
    }

    /// clear interrupt flags except ULP_WAKEUP and WKUP_STATUS as they need special handling
    pub fn clear_interrupt_flags(&self) {
        // clear COMBO_INTR_STATUS register
        self.registers.combo_intr_status.write(
            COMBO_INTR_STATUS::COMBO0_H2L.val(1)
                + COMBO_INTR_STATUS::COMBO1_H2L.val(1)
                + COMBO_INTR_STATUS::COMBO2_H2L.val(1)
                + COMBO_INTR_STATUS::COMBO3_H2L.val(1),
        );

        // clear KEY_INTR_STATUS register
        self.registers.key_intr_status.write(
            KEY_INTR_STATUS::PWRB_H2L.val(1)
                + KEY_INTR_STATUS::KEY0_IN_H2L.val(1)
                + KEY_INTR_STATUS::KEY1_IN_H2L.val(1)
                + KEY_INTR_STATUS::KEY2_IN_H2L.val(1)
                + KEY_INTR_STATUS::AC_PRESENT_H2L.val(1)
                + KEY_INTR_STATUS::EC_RST_L_H2L.val(1)
                + KEY_INTR_STATUS::FLASH_WP_L_H2L.val(1)
                + KEY_INTR_STATUS::PWRB_L2H.val(1)
                + KEY_INTR_STATUS::KEY0_IN_L2H.val(1)
                + KEY_INTR_STATUS::KEY1_IN_L2H.val(1)
                + KEY_INTR_STATUS::KEY2_IN_L2H.val(1)
                + KEY_INTR_STATUS::AC_PRESENT_L2H.val(1)
                + KEY_INTR_STATUS::EC_RST_L_L2H.val(1)
                + KEY_INTR_STATUS::FLASH_WP_L_L2H.val(1),
        );

        // clear peripheral's central interrupt flag
        self.registers.intr_state.write(INTR::EVENT_DETECTED.val(1));
    }

    pub fn handle_interrupt(&self, interrupt: SysRstCtrlInterrupt) {
        match interrupt {
            SysRstCtrlInterrupt::AonEventDetected => {
                let combo_state = self.registers.combo_intr_status.extract();
                let key_interrupt_state = self.registers.key_intr_status.extract();
                let input_pin_state = self.get_input_state();
                let ulp_wakeup = self.ulp_wakeup_detected();
                let wokeup = self.wakeup_detected();

                // 1st phase of ULP wakeup reset
                if ulp_wakeup {
                    self.clear_ulp_wakeup();
                }

                self.clear_interrupt_flags();

                self.client.map(|client| {
                    // if a combo detector's interrupt triggered then notify client
                    if combo_state.is_set(COMBO_INTR_STATUS::COMBO0_H2L) {
                        client.combo_detected(input_pin_state, SRCComboDetectorId::Zero);
                    }
                    if combo_state.is_set(COMBO_INTR_STATUS::COMBO1_H2L) {
                        client.combo_detected(input_pin_state, SRCComboDetectorId::One);
                    }
                    if combo_state.is_set(COMBO_INTR_STATUS::COMBO2_H2L) {
                        client.combo_detected(input_pin_state, SRCComboDetectorId::Two);
                    }
                    if combo_state.is_set(COMBO_INTR_STATUS::COMBO3_H2L) {
                        client.combo_detected(input_pin_state, SRCComboDetectorId::Three);
                    }
                    // if any key interrupt triggered, notify client
                    if key_interrupt_state.get() != 0 {
                        // determine which L2H transitions have triggered
                        let l2h = InMemoryRegister::new(0);
                        l2h.write(
                            SRCInputPinState::PowerButton
                                .val(key_interrupt_state.is_set(KEY_INTR_STATUS::PWRB_L2H) as u32)
                                + SRCInputPinState::Key0
                                    .val(key_interrupt_state.is_set(KEY_INTR_STATUS::KEY0_IN_L2H)
                                        as u32)
                                + SRCInputPinState::Key1
                                    .val(key_interrupt_state.is_set(KEY_INTR_STATUS::KEY1_IN_L2H)
                                        as u32)
                                + SRCInputPinState::Key2
                                    .val(key_interrupt_state.is_set(KEY_INTR_STATUS::KEY2_IN_L2H)
                                        as u32)
                                + SRCInputPinState::AcPresent.val(
                                    key_interrupt_state.is_set(KEY_INTR_STATUS::AC_PRESENT_L2H)
                                        as u32,
                                )
                                + SRCInputPinState::EcReset
                                    .val(key_interrupt_state.is_set(KEY_INTR_STATUS::EC_RST_L_L2H)
                                        as u32)
                                + SRCInputPinState::FlashWP.val(
                                    key_interrupt_state.is_set(KEY_INTR_STATUS::FLASH_WP_L_L2H)
                                        as u32,
                                ),
                        );
                        // determine which H2L transition have triggered
                        let h2l = InMemoryRegister::new(0);
                        l2h.write(
                            SRCInputPinState::PowerButton
                                .val(key_interrupt_state.is_set(KEY_INTR_STATUS::PWRB_H2L) as u32)
                                + SRCInputPinState::Key0
                                    .val(key_interrupt_state.is_set(KEY_INTR_STATUS::KEY0_IN_H2L)
                                        as u32)
                                + SRCInputPinState::Key1
                                    .val(key_interrupt_state.is_set(KEY_INTR_STATUS::KEY1_IN_H2L)
                                        as u32)
                                + SRCInputPinState::Key2
                                    .val(key_interrupt_state.is_set(KEY_INTR_STATUS::KEY2_IN_H2L)
                                        as u32)
                                + SRCInputPinState::AcPresent.val(
                                    key_interrupt_state.is_set(KEY_INTR_STATUS::AC_PRESENT_H2L)
                                        as u32,
                                )
                                + SRCInputPinState::EcReset
                                    .val(key_interrupt_state.is_set(KEY_INTR_STATUS::EC_RST_L_H2L)
                                        as u32)
                                + SRCInputPinState::FlashWP.val(
                                    key_interrupt_state.is_set(KEY_INTR_STATUS::FLASH_WP_L_H2L)
                                        as u32,
                                ),
                        );
                        client.key_interrupt(l2h.extract(), h2l.extract())
                    }
                    if wokeup {
                        client.wokeup(ulp_wakeup);
                    }
                });

                // 2nd phase of ULP wakeup reset
                if ulp_wakeup {
                    self.reset_ulp_wakeup();
                }
            }
        }
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

    /* PIN OVERRIDE and ALLOWED state */

    /// check if the HW is configured to allow a certain `pin` to be overriden to a certain `state` (logic level) in the PIN_ALLOWED_CTRL register
    pub fn is_output_pin_oveerride_allowed(&self, pin: SRCOutputPin, state: bool) -> bool {
        let pin_allowed_ctl = self.registers.pin_allowed_ctl.extract();
        match (pin, state) {
            (SRCOutputPin::BatDisable, true) => {
                pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::BAT_DISABLE_1)
            }
            (SRCOutputPin::BatDisable, false) => {
                pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::BAT_DISABLE_0)
            }
            (SRCOutputPin::EcReset, true) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::EC_RST_L_1),
            (SRCOutputPin::EcReset, false) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::EC_RST_L_0),
            (SRCOutputPin::Pwrb, true) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::PWRB_OUT_1),
            (SRCOutputPin::Pwrb, false) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::PWRB_OUT_0),
            (SRCOutputPin::Key0, true) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY0_OUT_1),
            (SRCOutputPin::Key0, false) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY0_OUT_0),
            (SRCOutputPin::Key1, true) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY1_OUT_1),
            (SRCOutputPin::Key1, false) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY1_OUT_0),
            (SRCOutputPin::Key2, true) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY2_OUT_1),
            (SRCOutputPin::Key2, false) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY2_OUT_0),
            (SRCOutputPin::Z3Wakeup, true) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::Z3_WAKEUP_1),
            (SRCOutputPin::Z3Wakeup, false) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::Z3_WAKEUP_0),
            (SRCOutputPin::FlashWP, true) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::FLASH_WP_L_1),
            (SRCOutputPin::FlashWP, false) => pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::FLASH_WP_L_0),
        }
    }

    /// get the output pin override pin state. Returns `None` if the pin is not being overriden, return `Some(state:bool)` with the state of the pin if it being overriden
    pub fn get_output_pin_override_state(&self, pin: SRCOutputPin) -> Option<bool> {
        let value_register = &self.registers.pin_out_value;
        let control_register = &self.registers.pin_out_ctl;
        match pin {
            SRCOutputPin::BatDisable => Self::autoblock_option_from_bits(
                control_register.is_set(PIN_OUT_CTL::BAT_DISABLE),
                value_register.is_set(PIN_OUT_VALUE::BAT_DISABLE),
            ),
            SRCOutputPin::EcReset => Self::autoblock_option_from_bits(
                control_register.is_set(PIN_OUT_CTL::EC_RST_L),
                value_register.is_set(PIN_OUT_VALUE::EC_RST_L),
            ),
            SRCOutputPin::Pwrb => Self::autoblock_option_from_bits(
                control_register.is_set(PIN_OUT_CTL::PWRB_OUT),
                value_register.is_set(PIN_OUT_VALUE::PWRB_OUT),
            ),
            SRCOutputPin::Key0 => Self::autoblock_option_from_bits(
                control_register.is_set(PIN_OUT_CTL::KEY0_OUT),
                value_register.is_set(PIN_OUT_VALUE::KEY0_OUT),
            ),
            SRCOutputPin::Key1 => Self::autoblock_option_from_bits(
                control_register.is_set(PIN_OUT_CTL::KEY1_OUT),
                value_register.is_set(PIN_OUT_VALUE::KEY1_OUT),
            ),
            SRCOutputPin::Key2 => Self::autoblock_option_from_bits(
                control_register.is_set(PIN_OUT_CTL::KEY2_OUT),
                value_register.is_set(PIN_OUT_VALUE::KEY2_OUT),
            ),
            SRCOutputPin::Z3Wakeup => Self::autoblock_option_from_bits(
                control_register.is_set(PIN_OUT_CTL::Z3_WAKEUP),
                value_register.is_set(PIN_OUT_VALUE::Z3_WAKEUP),
            ),
            SRCOutputPin::FlashWP => Self::autoblock_option_from_bits(
                control_register.is_set(PIN_OUT_CTL::FLASH_WP_L),
                value_register.is_set(PIN_OUT_VALUE::FLASH_WP_L),
            ),
        }
    }
}

impl<'a> OpenTitanSysRstr for SysRstCtrl<'a> {
    fn get_input_state(&self) -> kernel::hil::opentitan_sysrst::SRCInputPinStatus {
        let pin_value = self.registers.pin_in_value.get();
        SRCInputPinStatus::new(pin_value)
    }

    /// configure combo detector functionality
    /// conf._timer_ns will be divided by 5 as register content is in mutliple of 5us
    fn configure_combo_detector(
        &self,
        detector_id: SRCComboDetectorId,
        configuration: &SRCComboDetectorConfig,
    ) -> Result<(), ()> {
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

    fn get_combo_detector_configuration(
        &self,
        detector_id: SRCComboDetectorId,
    ) -> SRCComboDetectorConfig {
        let com_out_ctl = self.registers.com_out_ctl[detector_id as usize].extract();
        let com_sel_ctl = self.registers.com_sel_ctl[detector_id as usize].extract();
        let com_det_ctl = self.registers.com_det_ctl[detector_id as usize].extract();
        let com_pre_sel_ctl = self.registers.com_pre_sel_ctl[detector_id as usize].extract();
        let com_pre_det_ctl = self.registers.com_pre_det_ctl[detector_id as usize].extract();

        SRCComboDetectorConfig {
            precondition: SRCComboDetectorPins {
                ac_present: com_pre_sel_ctl.is_set(COM_PRE_SEL_CTL::AC_PRESENT_SEL_0),
                pwrb: com_pre_sel_ctl.is_set(COM_PRE_SEL_CTL::PWRB_IN_SEL_0),
                key0: com_pre_sel_ctl.is_set(COM_PRE_SEL_CTL::KEY0_IN_SEL_0),
                key1: com_pre_sel_ctl.is_set(COM_PRE_SEL_CTL::KEY1_IN_SEL_0),
                key2: com_pre_sel_ctl.is_set(COM_PRE_SEL_CTL::KEY2_IN_SEL_0),
            },
            precondition_time_us: com_pre_det_ctl.read(COM_PRE_DET_CTL::PRECONDITION_TIMER_0) * 5,
            condition: SRCComboDetectorPins {
                ac_present: com_sel_ctl.is_set(COM_SEL_CTL::AC_PRESENT_SEL_0),
                pwrb: com_sel_ctl.is_set(COM_SEL_CTL::PWRB_IN_SEL_0),
                key0: com_sel_ctl.is_set(COM_SEL_CTL::KEY0_IN_SEL_0),
                key1: com_sel_ctl.is_set(COM_SEL_CTL::KEY1_IN_SEL_0),
                key2: com_sel_ctl.is_set(COM_SEL_CTL::KEY2_IN_SEL_0),
            },
            condition_time_us: com_det_ctl.read(COM_DET_CTL::DETECTION_TIMER_0) * 5,
            action: SRCComboDetectorAction {
                rst_req: com_out_ctl.is_set(COM_OUT_CTL::RST_REQ_0),
                ec_rst: com_out_ctl.is_set(COM_OUT_CTL::EC_RST_0),
                interrupt: com_out_ctl.is_set(COM_OUT_CTL::INTERRUPT_0),
                bat_disable: com_out_ctl.is_set(COM_OUT_CTL::BAT_DISABLE_0),
            },
        }
    }

    fn configure_autoblock(&self, configuration: &SRCAutoblockConfig) -> Result<(), ()> {
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
                + AUTO_BLOCK_DEBOUNCE_CTL::AUTO_BLOCK_ENABLE.val(configuration.enable as u32),
        );
        Ok(())
    }

    fn get_autoblock_confiugration(&self) -> SRCAutoblockConfig {
        let out_ctl = self.registers.auto_block_out_ctl.extract();
        let debounce_ctl = self.registers.auto_block_debounce_ctl.extract();

        // extract bits from AUTO_BLOCK_DEBOUNCE_CTL
        let key0_out_sel = out_ctl.is_set(AUTO_BLOCK_OUT_CTL::KEY0_OUT_SEL);
        let key0_out_value = out_ctl.is_set(AUTO_BLOCK_OUT_CTL::KEY0_OUT_VALUE);
        let key1_out_sel = out_ctl.is_set(AUTO_BLOCK_OUT_CTL::KEY1_OUT_SEL);
        let key1_out_value = out_ctl.is_set(AUTO_BLOCK_OUT_CTL::KEY1_OUT_VALUE);
        let key2_out_sel = out_ctl.is_set(AUTO_BLOCK_OUT_CTL::KEY2_OUT_SEL);
        let key2_out_value = out_ctl.is_set(AUTO_BLOCK_OUT_CTL::KEY2_OUT_VALUE);

        // construct configuration from extracted bits
        SRCAutoblockConfig {
            pwrb_debounce_timer_us: debounce_ctl.read(AUTO_BLOCK_DEBOUNCE_CTL::DEBOUNCE_TIMER) * 5,
            block_key0: Self::autoblock_option_from_bits(key0_out_sel, key0_out_value),
            block_key1: Self::autoblock_option_from_bits(key1_out_sel, key1_out_value),
            block_key2: Self::autoblock_option_from_bits(key2_out_sel, key2_out_value),
            enable: debounce_ctl.is_set(AUTO_BLOCK_DEBOUNCE_CTL::AUTO_BLOCK_ENABLE),
        }
    }

    fn configure_keyinterrupt(&self, configuration: &SRCKeyInterruptConfig) -> Result<(), ()> {
        self.registers.key_intr_ctl.write(
            KEY_INTR_CTL::FLASH_WP_L_H2L.val(configuration.flash_wp_h2l as u32)
                + KEY_INTR_CTL::FLASH_WP_L_L2H.val(configuration.flash_wp_l2h as u32)
                + KEY_INTR_CTL::EC_RST_L_H2L.val(configuration.ec_reset_h2l as u32)
                + KEY_INTR_CTL::EC_RST_L_L2H.val(configuration.ec_reset_l2h as u32)
                + KEY_INTR_CTL::AC_PRESENT_H2L.val(configuration.ac_present_h2l as u32)
                + KEY_INTR_CTL::AC_PRESENT_L2H.val(configuration.ac_present_l2h as u32)
                + KEY_INTR_CTL::KEY2_IN_H2L.val(configuration.key2_h2l as u32)
                + KEY_INTR_CTL::KEY2_IN_L2H.val(configuration.key2_l2h as u32)
                + KEY_INTR_CTL::KEY1_IN_H2L.val(configuration.key1_h2l as u32)
                + KEY_INTR_CTL::KEY1_IN_L2H.val(configuration.key1_l2h as u32)
                + KEY_INTR_CTL::KEY0_IN_H2L.val(configuration.key0_h2l as u32)
                + KEY_INTR_CTL::KEY0_IN_L2H.val(configuration.key0_l2h as u32)
                + KEY_INTR_CTL::PWRB_IN_H2L.val(configuration.pwrb_h2l as u32)
                + KEY_INTR_CTL::PWRB_IN_L2H.val(configuration.pwrb_l2h as u32),
        );
        Ok(())
    }

    fn get_keyinterrupt_confiugration(&self) -> SRCKeyInterruptConfig {
        let intr_ctl = self.registers.key_intr_ctl.extract();

        SRCKeyInterruptConfig {
            pwrb_h2l: intr_ctl.is_set(KEY_INTR_CTL::PWRB_IN_H2L),
            pwrb_l2h: intr_ctl.is_set(KEY_INTR_CTL::PWRB_IN_L2H),
            key0_h2l: intr_ctl.is_set(KEY_INTR_CTL::KEY0_IN_H2L),
            key0_l2h: intr_ctl.is_set(KEY_INTR_CTL::KEY0_IN_L2H),
            key1_h2l: intr_ctl.is_set(KEY_INTR_CTL::KEY1_IN_H2L),
            key1_l2h: intr_ctl.is_set(KEY_INTR_CTL::KEY1_IN_L2H),
            key2_h2l: intr_ctl.is_set(KEY_INTR_CTL::KEY2_IN_H2L),
            key2_l2h: intr_ctl.is_set(KEY_INTR_CTL::KEY2_IN_L2H),
            ac_present_h2l: intr_ctl.is_set(KEY_INTR_CTL::AC_PRESENT_H2L),
            ac_present_l2h: intr_ctl.is_set(KEY_INTR_CTL::AC_PRESENT_L2H),
            ec_reset_h2l: intr_ctl.is_set(KEY_INTR_CTL::EC_RST_L_H2L),
            ec_reset_l2h: intr_ctl.is_set(KEY_INTR_CTL::EC_RST_L_L2H),
            flash_wp_h2l: intr_ctl.is_set(KEY_INTR_CTL::FLASH_WP_L_H2L),
            flash_wp_l2h: intr_ctl.is_set(KEY_INTR_CTL::FLASH_WP_L_L2H),
        }
    }

    fn configure_pin_invertion(&self, config: &SRCPinInversionConfig) -> Result<(), ()> {
        self.registers.key_invert_ctl.write(
            KEY_INVERT_CTL::Z3_WAKEUP.val(config.z3_wakeup_output as u32)
                + KEY_INVERT_CTL::LID_OPEN.val(config.lid_open_input as u32)
                + KEY_INVERT_CTL::BAT_DISABLE.val(config.bat_disable_output as u32)
                + KEY_INVERT_CTL::AC_PRESENT.val(config.ac_present_input as u32)
                + KEY_INVERT_CTL::PWRB_OUT.val(config.pwrb_output as u32)
                + KEY_INVERT_CTL::PWRB_IN.val(config.pwrb_input as u32)
                + KEY_INVERT_CTL::KEY2_OUT.val(config.key2_output as u32)
                + KEY_INVERT_CTL::KEY2_IN.val(config.key2_input as u32)
                + KEY_INVERT_CTL::KEY1_OUT.val(config.key1_output as u32)
                + KEY_INVERT_CTL::KEY1_IN.val(config.key1_input as u32)
                + KEY_INVERT_CTL::KEY0_OUT.val(config.key0_output as u32)
                + KEY_INVERT_CTL::KEY0_IN.val(config.key0_input as u32),
        );
        Ok(())
    }

    fn get_pin_invertion_configuration(&self) -> SRCPinInversionConfig {
        let key_invert_ctl = self.registers.key_invert_ctl.extract();
        SRCPinInversionConfig {
            z3_wakeup_output: key_invert_ctl.is_set(KEY_INVERT_CTL::Z3_WAKEUP),
            lid_open_input: key_invert_ctl.is_set(KEY_INVERT_CTL::LID_OPEN),
            bat_disable_output: key_invert_ctl.is_set(KEY_INVERT_CTL::BAT_DISABLE),
            ac_present_input: key_invert_ctl.is_set(KEY_INVERT_CTL::AC_PRESENT),
            pwrb_output: key_invert_ctl.is_set(KEY_INVERT_CTL::PWRB_OUT),
            pwrb_input: key_invert_ctl.is_set(KEY_INVERT_CTL::PWRB_IN),
            key0_output: key_invert_ctl.is_set(KEY_INVERT_CTL::KEY0_OUT),
            key0_input: key_invert_ctl.is_set(KEY_INVERT_CTL::KEY0_IN),
            key1_output: key_invert_ctl.is_set(KEY_INVERT_CTL::KEY1_OUT),
            key1_input: key_invert_ctl.is_set(KEY_INVERT_CTL::KEY1_IN),
            key2_output: key_invert_ctl.is_set(KEY_INVERT_CTL::KEY2_OUT),
            key2_input: key_invert_ctl.is_set(KEY_INVERT_CTL::KEY2_IN),
        }
    }

    fn configure_allowed_override_pin_states(
        &self,
        configuration: &SRCAllowedPinConfig,
    ) -> Result<(), ()> {
        if self.is_configuration_locked() {
            return Err(());
        }
        self.registers.pin_allowed_ctl.write(
            PIN_ALLOWED_CTL::BAT_DISABLE_0.val(configuration.bat_disable_0 as u32)
                + PIN_ALLOWED_CTL::BAT_DISABLE_1.val(configuration.bat_disable_1 as u32)
                + PIN_ALLOWED_CTL::EC_RST_L_0.val(configuration.ec_reset_0 as u32)
                + PIN_ALLOWED_CTL::EC_RST_L_1.val(configuration.ec_reset_1 as u32)
                + PIN_ALLOWED_CTL::PWRB_OUT_0.val(configuration.pwrb_0 as u32)
                + PIN_ALLOWED_CTL::PWRB_OUT_1.val(configuration.pwrb_1 as u32)
                + PIN_ALLOWED_CTL::KEY0_OUT_0.val(configuration.key0_0 as u32)
                + PIN_ALLOWED_CTL::KEY0_OUT_1.val(configuration.key0_1 as u32)
                + PIN_ALLOWED_CTL::KEY1_OUT_0.val(configuration.key1_0 as u32)
                + PIN_ALLOWED_CTL::KEY1_OUT_1.val(configuration.key1_1 as u32)
                + PIN_ALLOWED_CTL::KEY2_OUT_0.val(configuration.key2_0 as u32)
                + PIN_ALLOWED_CTL::KEY2_OUT_1.val(configuration.key2_1 as u32)
                + PIN_ALLOWED_CTL::Z3_WAKEUP_0.val(configuration.z3_wakeup_0 as u32)
                + PIN_ALLOWED_CTL::Z3_WAKEUP_1.val(configuration.z3_wakeup_1 as u32)
                + PIN_ALLOWED_CTL::FLASH_WP_L_0.val(configuration.flash_wp_0 as u32)
                + PIN_ALLOWED_CTL::FLASH_WP_L_1.val(configuration.flash_wp_1 as u32),
        );
        Ok(())
    }

    fn get_allowed_override_pin_state_confiugration(&self) -> SRCAllowedPinConfig {
        let pin_allowed_ctl = self.registers.pin_allowed_ctl.extract();
        SRCAllowedPinConfig {
            bat_disable_0: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::BAT_DISABLE_0),
            bat_disable_1: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::BAT_DISABLE_1),
            ec_reset_0: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::EC_RST_L_0),
            ec_reset_1: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::EC_RST_L_1),
            pwrb_0: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::PWRB_OUT_0),
            pwrb_1: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::PWRB_OUT_1),
            key0_0: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY0_OUT_0),
            key0_1: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY0_OUT_1),
            key1_0: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY1_OUT_0),
            key1_1: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY1_OUT_1),
            key2_0: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY2_OUT_0),
            key2_1: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::KEY2_OUT_1),
            z3_wakeup_0: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::Z3_WAKEUP_0),
            z3_wakeup_1: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::Z3_WAKEUP_1),
            flash_wp_0: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::FLASH_WP_L_0),
            flash_wp_1: pin_allowed_ctl.is_set(PIN_ALLOWED_CTL::FLASH_WP_L_1),
        }
    }

    /// configure the HW to override a certain `pin` to a certain `state` (logic level). This will fail if the HW is not configured to allow this change
    fn override_output_pin(&self, pin: SRCOutputPin, state: Option<bool>) {
        let value = state.unwrap_or(false) as u32;
        let control = state.is_some() as u32;

        let (value_field, control_field) = match pin {
            SRCOutputPin::Pwrb => (PIN_OUT_VALUE::PWRB_OUT, PIN_OUT_CTL::PWRB_OUT),
            SRCOutputPin::BatDisable => (PIN_OUT_VALUE::BAT_DISABLE, PIN_OUT_CTL::BAT_DISABLE),
            SRCOutputPin::EcReset => (PIN_OUT_VALUE::EC_RST_L, PIN_OUT_CTL::EC_RST_L),
            SRCOutputPin::Key0 => (PIN_OUT_VALUE::KEY0_OUT, PIN_OUT_CTL::KEY0_OUT),
            SRCOutputPin::Key1 => (PIN_OUT_VALUE::KEY1_OUT, PIN_OUT_CTL::KEY1_OUT),
            SRCOutputPin::Key2 => (PIN_OUT_VALUE::KEY2_OUT, PIN_OUT_CTL::KEY2_OUT),
            SRCOutputPin::Z3Wakeup => (PIN_OUT_VALUE::Z3_WAKEUP, PIN_OUT_CTL::Z3_WAKEUP),
            SRCOutputPin::FlashWP => (PIN_OUT_VALUE::FLASH_WP_L, PIN_OUT_CTL::FLASH_WP_L),
        };

        self.registers.pin_out_value.modify(value_field.val(value));
        self.registers
            .pin_out_ctl
            .modify(control_field.val(control));
    }

    fn configure_wakeup(&self, configuration: &SRCWakeupConfig) {
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

    fn get_wakeup_configuration(&self) -> SRCWakeupConfig {
        let ac_present_debounce_timer_us = (self
            .registers
            .ulp_ac_debounce_ctl
            .read(ULP_AC_DEBOUNCE_CTL::ULP_AC_DEBOUNCE_TIMER)
            as u16)
            * 5;

        let pwrb_debounce_timer_us = (self
            .registers
            .ulp_pwrb_debounce_ctl
            .read(ULP_PWRB_DEBOUNCE_CTL::ULP_PWRB_DEBOUNCE_TIMER)
            as u16)
            * 5;

        let lid_open_debounce_timer_us = (self
            .registers
            .ulp_lid_debounce_ctl
            .read(ULP_LID_DEBOUNCE_CTL::ULP_LID_DEBOUNCE_TIMER)
            as u16)
            * 5;

        let enabled = self.registers.ulp_ctl.is_set(ULP_CTL::ULP_ENABLE);

        SRCWakeupConfig {
            ac_present_debounce_timer_us,
            pwrb_debounce_timer_us,
            lid_open_debounce_timer_us,
            enabled,
        }
    }

    /// configure input debuounce timer that affercts Key Interrupt and Combo Detector
    fn configure_debouncetimer(&self, duration_us: u16) -> Result<(), ()> {
        if self.is_configuration_locked() {
            return Err(());
        }
        self.registers
            .key_intr_debounce_ctl
            .write(KEY_INTR_DEBOUNCE_CTL::DEBOUNCE_TIMER.val(u16::div_ceil(duration_us, 5) as u32));
        Ok(())
    }

    fn get_debouncetimer_configuration(&self) -> u32 {
        self.registers
            .key_intr_debounce_ctl
            .read(KEY_INTR_DEBOUNCE_CTL::DEBOUNCE_TIMER)
            * 5
    }

    fn lock_configuration(&self) {
        self.registers.regwen.write(REGWEN::WRITE_EN.val(0));
    }
}

#[cfg(feature = "test_sysrst_ctrl")]
pub mod tests {
    use kernel::hil::{
        gpio::{self},
        opentitan_sysrst::{
            OpenTitanSysRstr, SRCAutoblockConfig, SRCComboDetectorConfig, SRCComboDetectorId,
            SRCKeyInterruptConfig, SRCPinInversionConfig,
        },
        time::{Alarm, AlarmClient},
    };

    use super::{SRCAllowedPinConfig, SysRstCtrl, KEY_INTR_STATUS};

    /// wait for a small amount of time for a quick HW process to finish
    fn blocking_wait(ticks: u32) {
        for _ in 0..ticks {
            unsafe {
                core::arch::asm!("nop");
            }
        }
    }

    /// CPU's clock is max 100MHz, this peripheral's clock is 800kHz. This function ensures that the CPU waits enough such that the peripheral's clock can tick at least once (100MHz/800kHz = 125 + some margin = 150 ) in order for it to process the inputs
    fn clock_domain_sync() {
        blocking_wait(150);
    }

    /// test that the (physical) wiring was done such that:
    ///     key0_force GPIO Ouput is connected to SysRst_Ctrl's key0_input,
    ///     pwrb_force GPIO Output is connected to SysRst_Ctrl's pwrb_input,
    ///     key0_sense GPIO Input is connected to SysRst_Ctrl's key0_output
    /// the function applies all force signal combinations and checks that the peripheral correctly receives the input signals, the sense signal correctly recievs the peripheral's output
    pub fn test_wiring<INPUT: gpio::Input, OUTPUT: gpio::Output>(
        sysrst_ctrl: &SysRstCtrl,
        key0_sense: &INPUT,
        key0_force: &OUTPUT,
        pwrb_force: &OUTPUT,
    ) {
        // preapare initial state with both force signals as low
        key0_force.clear();
        pwrb_force.clear();

        // check that the peripheral sees both signals as low
        assert!(!sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Key0));
        assert!(!sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Pwrb));

        // check that the peripheral (without any inversion) generates the correct key0_output (low) and the GPIO sees that signal as low
        assert!(
            !key0_sense.read(),
            "Key0_force = 0 should passthough to Key0_output = 0"
        );

        // set force signals as low, high
        key0_force.clear();
        pwrb_force.set();

        // check that the peripheral correctly receives the signals
        assert!(!sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Key0));
        assert!(sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Pwrb));

        // check that the sense signal correctly receives the peripheral's output
        assert!(
            !key0_sense.read(),
            "Key0_force = 0 should not change when pwrb is changed"
        );

        // set force signals as high, high
        key0_force.set();
        pwrb_force.set();

        // check that the peripheral correctly receives the signals
        assert!(sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Key0));
        assert!(sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Pwrb));

        // check that the sense signal correctly receives the peripheral's output
        assert!(
            key0_sense.read(),
            "Key0_force = 1 should passthrough to Key0_output = 1"
        );

        // set force signals as high, low
        key0_force.set();
        pwrb_force.clear();

        // check that the peripheral correctly receives the signals
        assert!(sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Key0));
        assert!(!sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Pwrb));

        // check that the sense signal correctly receives the peripheral's output
        assert!(
            key0_sense.read(),
            "Key0_force = 1 should not change when pwrb is changed"
        );

        key0_force.clear();
        pwrb_force.clear();

        kernel::debug!("R test_wiring passed");
    }

    /// test ComboDetector functions by:
    ///     configuring the relevant module to detect a certain input signal combination
    ///     apply those input signals
    ///     checking if the module detected the applied inputs
    ///     reading back the confiugration from HW registers and comparing them to the initial (desired) configuration
    pub fn test_combodetector<INPUT: gpio::Input, OUTPUT: gpio::Output>(
        sysrst_ctrl: &SysRstCtrl,
        key0_sense: &INPUT,
        key0_force: &OUTPUT,
        pwrb_force: &OUTPUT,
    ) {
        // define a Combo Detector confiugration for detecting a high to low transition on both PwrB and Key0 signals, both signals stay in that state for 5 us, no precondition, an interrupt is genereated when the condition is detected
        let configuration = super::SRCComboDetectorConfig {
            precondition: super::SRCComboDetectorPins {
                ac_present: false,
                pwrb: false,
                key0: false,
                key1: false,
                key2: false,
            },
            precondition_time_us: 5,
            condition: super::SRCComboDetectorPins {
                ac_present: false,
                pwrb: true,
                key0: true,
                key1: false,
                key2: false,
            },
            condition_time_us: 5,
            action: super::SRCComboDetectorAction {
                rst_req: false,
                ec_rst: false,
                interrupt: true,
                bat_disable: false,
            },
        };

        sysrst_ctrl.enable_interrupts();

        // Combo Detector and Key Interrupt share the common debounce timer
        sysrst_ctrl
            .configure_debouncetimer(5)
            .expect("HW registers are locked");

        // confiugre the combo detector
        let configure_result =
            sysrst_ctrl.configure_combo_detector(SRCComboDetectorId::Zero, &configuration);

        // check that the configuration was reported as done
        assert_eq!(configure_result, Ok(()));

        // prepare initial state
        pwrb_force.set();
        key0_force.set();

        // check that the Key0 signal passed through
        assert!(key0_sense.read());

        // generate a high to low transition on Pwrb and Key0
        key0_force.clear();
        pwrb_force.clear();

        // wait for the combo detector timers to filter and detect the transition
        clock_domain_sync();

        // check that the ComboDetector has detected only this transition and has generated the interrupt (clear the interrupt flags after that)
        assert!(sysrst_ctrl.is_combo_detector_interrupt_triggered(SRCComboDetectorId::Zero));
        assert!(!sysrst_ctrl.is_combo_detector_interrupt_triggered(SRCComboDetectorId::One));
        assert!(!sysrst_ctrl.is_combo_detector_interrupt_triggered(SRCComboDetectorId::Two));
        assert!(!sysrst_ctrl.is_combo_detector_interrupt_triggered(SRCComboDetectorId::Three));
        sysrst_ctrl.clear_combo_detector_interrupt_status(SRCComboDetectorId::Zero);

        // read back the ComboDetector's confiugration
        let readback_configuration =
            sysrst_ctrl.get_combo_detector_configuration(SRCComboDetectorId::Zero);

        // check that the readback confiugration is identical to the inital configuration
        assert_eq!(
            configuration, readback_configuration,
            "readback confiugration differs from initial confiugration"
        );

        // check that the readback debounce timer confiugration is the same as the initial (desired) confiugration
        let readback_debouncetimer = sysrst_ctrl.get_debouncetimer_configuration();
        assert_eq!(readback_debouncetimer, 5);

        // deconfigure the ComboDetector
        sysrst_ctrl
            .configure_combo_detector(
                SRCComboDetectorId::Zero,
                &(SRCComboDetectorConfig::default()),
            )
            .expect("");
    }

    /// test Key Interrupt feature by configuring the relevant module and testing that changes to Key0 and PwrB are detected in the Key Interrupt module
    pub fn test_keyinterrupt<INPUT: gpio::Input, OUTPUT: gpio::Output>(
        sysrst_ctrl: &SysRstCtrl,
        _key0_sense: &INPUT,
        key0_force: &OUTPUT,
        pwrb_force: &OUTPUT,
    ) {
        //define a module configuration to detect any H2L or L2H changes on relevant input pins
        let configuration = super::SRCKeyInterruptConfig {
            pwrb_h2l: true,
            pwrb_l2h: true,
            key0_h2l: true,
            key0_l2h: true,
            key1_h2l: false,
            key1_l2h: false,
            key2_h2l: false,
            key2_l2h: false,
            ac_present_h2l: false,
            ac_present_l2h: false,
            ec_reset_h2l: false,
            ec_reset_l2h: false,
            flash_wp_h2l: false,
            flash_wp_l2h: false,
        };

        // set initial pin state to (0,0) and clear relevant key interrupt status register as a possible high to low interrupt could have been generated
        key0_force.clear();
        pwrb_force.clear();
        sysrst_ctrl.key_interrupt_clear(KEY_INTR_STATUS::KEY0_IN_H2L);
        sysrst_ctrl.key_interrupt_clear(KEY_INTR_STATUS::PWRB_H2L);
        sysrst_ctrl.enable_interrupts();

        sysrst_ctrl
            .configure_keyinterrupt(&configuration)
            .expect("peripheral is locked");

        // Combo Detector and Key Interrupt share the common debounce timer
        sysrst_ctrl
            .configure_debouncetimer(5)
            .expect("HW registers are locked");

        clock_domain_sync();

        // force a L2H transition on Key0 pin
        key0_force.set();

        clock_domain_sync();

        // check that only the relevant Key0 transition was triggered
        let key_interrupt_status = sysrst_ctrl.key_interrupt_status();
        assert!(key_interrupt_status.is_set(KEY_INTR_STATUS::KEY0_IN_L2H));

        // check that only Key0 transition was triggered
        assert_eq!(
            key_interrupt_status.get(),
            0x100,
            "another interrupt flag is set"
        );

        // check that a wakeup trigger was detected
        assert!(sysrst_ctrl.wakeup_detected());

        // clear state for next test
        sysrst_ctrl.key_interrupt_clear(KEY_INTR_STATUS::KEY0_IN_L2H);
        sysrst_ctrl.clear_wakeup();

        // force a L2H transition on PwrB pin and a H2L transition on Key0 pin
        key0_force.clear();
        pwrb_force.set();

        clock_domain_sync();
        let key_interrupt_status = sysrst_ctrl.key_interrupt_status();
        // check that the relevant transitions were triggered
        assert!(key_interrupt_status.matches_all(
            KEY_INTR_STATUS::PWRB_L2H.val(1)
                + KEY_INTR_STATUS::PWRB_H2L.val(0)
                + KEY_INTR_STATUS::KEY0_IN_H2L.val(1)
                + KEY_INTR_STATUS::KEY0_IN_L2H.val(0)
        ));

        // check that only the relevant transitions were triggered
        assert!(
            key_interrupt_status.matches_all(
                KEY_INTR_STATUS::PWRB_H2L.val(0)
                    + KEY_INTR_STATUS::KEY0_IN_H2L.val(1)
                    + KEY_INTR_STATUS::KEY1_IN_H2L.val(0)
                    + KEY_INTR_STATUS::KEY2_IN_H2L.val(0)
                    + KEY_INTR_STATUS::AC_PRESENT_H2L.val(0)
                    + KEY_INTR_STATUS::EC_RST_L_H2L.val(0)
                    + KEY_INTR_STATUS::FLASH_WP_L_H2L.val(0)
                    + KEY_INTR_STATUS::PWRB_L2H.val(1)
                    + KEY_INTR_STATUS::KEY0_IN_L2H.val(0)
                    + KEY_INTR_STATUS::KEY1_IN_L2H.val(0)
                    + KEY_INTR_STATUS::KEY2_IN_L2H.val(0)
                    + KEY_INTR_STATUS::AC_PRESENT_L2H.val(0)
                    + KEY_INTR_STATUS::EC_RST_L_L2H.val(0)
                    + KEY_INTR_STATUS::FLASH_WP_L_L2H.val(0)
            ),
            "{:x}",
            key_interrupt_status.get()
        );

        // // read back the KeyInterrupt's confiugration
        let readback_confiugration = sysrst_ctrl.get_keyinterrupt_confiugration();

        // check that the readback confiugration is identical to the inital configuration
        assert_eq!(readback_confiugration, configuration);

        // clear state for next test
        sysrst_ctrl
            .configure_keyinterrupt(&SRCKeyInterruptConfig::default())
            .expect("peripheral is locked");
        sysrst_ctrl.key_interrupt_clear(KEY_INTR_STATUS::KEY0_IN_H2L);
        sysrst_ctrl.key_interrupt_clear(KEY_INTR_STATUS::PWRB_L2H);
        sysrst_ctrl.clear_wakeup();
        key0_force.clear();
        pwrb_force.clear();
    }

    /// test Autoblock functions by:
    ///     configuring HW to block key0 to high when Pwrb is pressed for >=1500us
    ///     generating triggering condition on Pwrb
    ///     check if key0_output is blocked to high by HW even if Key0_input is changed
    ///     read back configuration from HW registers and check if they match initial configuration
    ///     disable HW block by reconfiguring to a default state
    fn test_autoblock<INPUT: gpio::Input, OUTPUT: gpio::Output>(
        sysrst_ctrl: &SysRstCtrl,
        key0_sense: &INPUT,
        key0_force: &OUTPUT,
        pwrb_force: &OUTPUT,
    ) {
        // define a module configuration such that if PwrB is pressed for >=1500us then Key0 is forced to high
        let configuration = super::SRCAutoblockConfig {
            pwrb_debounce_timer_us: 1,
            block_key0: Some(true),
            block_key1: None,
            block_key2: None,
            enable: true,
        };

        // set initial pin state to low, high
        // autoblock waits for a H to L transition on PwrB
        key0_force.clear();
        pwrb_force.set();

        sysrst_ctrl
            .configure_autoblock(&configuration)
            .expect("peripheral is locked");

        clock_domain_sync();

        // check that key0 signal was not changed by autoblock being enabled
        assert!(
            !key0_sense.read(),
            "Key0 should not change when Autoblock is configured"
        );

        // set PwrB to low, triggering Autoblock
        pwrb_force.clear();

        blocking_wait(10000);

        // check that even if Key0 input is low, Key0 output is high because of the autoblock feature
        assert!(
            key0_sense.read(),
            "Key0 should have been blocked and forced to high"
        );

        // clear state for next test
        sysrst_ctrl
            .configure_autoblock(&SRCAutoblockConfig::default())
            .expect("returning to the disabled state should work");
        key0_force.clear();
        pwrb_force.clear();
    }

    /// test Invertion functions by:
    ///     configuring HW to invert at input/output
    ///     changing input state
    ///     checking output state is changed according to input and inversion configuration
    ///     read back configuration from HW registers, check if they match desired configuration
    fn test_key0_invertion<INPUT: gpio::Input, OUTPUT: gpio::Output>(
        sysrst_ctrl: &SysRstCtrl,
        key0_sense: &INPUT,
        key0_force: &OUTPUT,
        _pwrb_force: &OUTPUT,
    ) {
        // define a module confiugration that inverts the key0 signal at input
        let input_invert_configuration = SRCPinInversionConfig {
            z3_wakeup_output: false,
            lid_open_input: false,
            bat_disable_output: false,
            ac_present_input: false,
            pwrb_output: false,
            pwrb_input: false,
            key0_output: false,
            key0_input: true,
            key1_output: false,
            key1_input: false,
            key2_output: false,
            key2_input: false,
        };

        // check Key0 pass through
        key0_force.clear();
        clock_domain_sync();
        assert!(!sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Key0));
        assert!(!key0_sense.read());

        key0_force.set();
        clock_domain_sync();
        assert!(sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Key0));
        assert!(key0_sense.read());

        // invert Key0 with input inversion
        sysrst_ctrl
            .configure_pin_invertion(&(input_invert_configuration))
            .expect("peripheral is locked");

        // check Key0 with input inversion (after input pin state register)
        key0_force.set();
        clock_domain_sync();
        assert!(sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Key0));
        assert!(!key0_sense.read());

        key0_force.clear();
        clock_domain_sync();
        assert!(!sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Key0));
        assert!(key0_sense.read());

        // readback the pin inversion confiugration from registers
        let readback_confiugration = sysrst_ctrl.get_pin_invertion_configuration();

        // compare that the readback confiugration is the same as the initial configuration
        assert_eq!(readback_confiugration, input_invert_configuration);

        // define a module confiugration that inverts the key0 signal at input
        let output_invert_confiugration = SRCPinInversionConfig {
            z3_wakeup_output: false,
            lid_open_input: false,
            bat_disable_output: false,
            ac_present_input: false,
            pwrb_output: false,
            pwrb_input: false,
            key0_output: true,
            key0_input: false,
            key1_output: false,
            key1_input: false,
            key2_output: false,
            key2_input: false,
        };

        sysrst_ctrl
            .configure_pin_invertion(&(output_invert_confiugration))
            .expect("peripheral is not locked");

        // check Key0 with output inversion
        key0_force.set();
        clock_domain_sync();
        assert!(sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Key0));
        assert!(!key0_sense.read());

        key0_force.clear();
        clock_domain_sync();
        assert!(!sysrst_ctrl.get_input_pin_state(&crate::sysrst_ctrl::SRCInputPin::Key0));
        assert!(key0_sense.read());

        // readback the pin inversion confiugration from registers
        let readback_confiugration = sysrst_ctrl.get_pin_invertion_configuration();

        // compare that the readback confiugration is the same as the initial configuration
        assert_eq!(readback_confiugration, output_invert_confiugration);

        // disable pin inversion configuration
        sysrst_ctrl
            .configure_pin_invertion(&SRCPinInversionConfig::default())
            .expect("peripheral is locked");
    }

    /// test pin override functionality by:
    ///     configuring HW to allow Key0 to be overriden to high
    ///     trigger Key0 to be overriden to low, HW should change output signal
    ///     trigger Key0 to be overriden to high, HW should overriden Key0_output to high
    ///     change input signals, check output signals (Key0 should be high regardless of input)
    ///     trigger Key0 to not be overriden
    ///     change input signals, check output signals (Key0_output should track Key0_input)
    ///     read back configuration from HW registers
    fn test_pin_override<INPUT: gpio::Input, OUTPUT: gpio::Output>(
        sysrst_ctrl: &SysRstCtrl,
        key0_sense: &INPUT,
        key0_force: &OUTPUT,
        pwrb_force: &OUTPUT,
    ) {
        // configure Allowed Override feature to allow overriding key0 to high
        let configuration = SRCAllowedPinConfig {
            bat_disable_0: false,
            bat_disable_1: false,
            ec_reset_0: false,
            ec_reset_1: false,
            pwrb_0: false,
            pwrb_1: false,
            key0_0: false,
            key0_1: true,
            key1_0: false,
            key1_1: false,
            key2_0: false,
            key2_1: false,
            z3_wakeup_0: false,
            z3_wakeup_1: false,
            flash_wp_0: false,
            flash_wp_1: false,
        };

        // configure inital state for Key0 low
        key0_force.clear();
        pwrb_force.clear();

        sysrst_ctrl
            .configure_allowed_override_pin_states(&configuration)
            .expect("configuration should have been posible");

        // override Key to low (should not be allowed)
        sysrst_ctrl.override_output_pin(super::SRCOutputPin::Key0, Some(false));
        assert!(!key0_sense.read());

        // change Key0 input to high, override circuit should have no effect
        key0_force.set();
        assert!(key0_sense.read());

        // override Key0 to high, output should be high
        sysrst_ctrl.override_output_pin(super::SRCOutputPin::Key0, Some(true));
        assert!(key0_sense.read());

        // Key0 input low, Key0 override should force output to high
        key0_force.clear();
        clock_domain_sync();
        assert!(key0_sense.read());

        // disable Key0 override
        sysrst_ctrl.override_output_pin(super::SRCOutputPin::Key0, None);

        // check that when Key0_input is low the output tracks
        key0_force.clear();
        clock_domain_sync();
        assert!(!key0_sense.read());

        // check that when Key0_input is high the output tracks
        key0_force.set();
        clock_domain_sync();
        assert!(key0_sense.read());

        // readback the pin inversion confiugration from registers
        let readback_configuration = sysrst_ctrl.get_allowed_override_pin_state_confiugration();

        assert_eq!(readback_configuration, configuration);

        // disable override
        sysrst_ctrl
            .configure_allowed_override_pin_states(&SRCAllowedPinConfig::default())
            .expect("disabling should work");
    }

    pub fn test_all<INPUT: gpio::Input, OUTPUT: gpio::Output>(
        sysrst_ctrl: &SysRstCtrl,
        key0_sense: &INPUT,
        key0_force: &OUTPUT,
        pwrb_force: &OUTPUT,
    ) {
        test_wiring(sysrst_ctrl, key0_sense, key0_force, pwrb_force);
        test_combodetector(sysrst_ctrl, key0_sense, key0_force, pwrb_force);
        test_keyinterrupt(sysrst_ctrl, key0_sense, key0_force, pwrb_force);
        test_autoblock(sysrst_ctrl, key0_sense, key0_force, pwrb_force);
        test_key0_invertion(sysrst_ctrl, key0_sense, key0_force, pwrb_force);
        test_pin_override(sysrst_ctrl, key0_sense, key0_force, pwrb_force);
        kernel::debug!("SystemReset_Ctrl tests PASSED");
    }

    pub struct Tests<'a, A: Alarm<'a>> {
        alarm: &'a A,
        sysrst_ctrl: &'a SysRstCtrl<'a>,
    }

    impl<'a, A: Alarm<'a>> Tests<'a, A> {
        pub fn new(sysrst_ctrl: &'a SysRstCtrl<'a>, alarm: &'a A) -> Self {
            Self { alarm, sysrst_ctrl }
        }
    }

    impl<'a, A: Alarm<'a>> AlarmClient for Tests<'a, A> {
        fn alarm(&self) {}
    }
}
