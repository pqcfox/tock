// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header has to be included to be able to submit it to Tock
// It is up to ZeroRISC to decide if it keeps this header or not
//

//! HIL for interacting with SysRstr_Ctrl on OpenTitan

use tock_registers::{register_bitfields, LocalRegisterCopy};

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
    BatDisable,
    EcReset,
    Pwrb,
    Key0,
    Key1,
    Key2,
    Z3Wakeup,
    FlashWP,
}

register_bitfields![u32,
pub SRCInputPinState [
    PowerButton 0,
    Key0 1,
    Key1 2,
    Key2 3,
    LidOpen 4,
    AcPresent 5,
    EcReset 6,
    FlashWP 7
]];

pub type SRCInputPinStatus = LocalRegisterCopy<u32, SRCInputPinState::Register>;

/// Struct that holds the Key Interrupt feature's configuration
#[derive(Debug, PartialEq, Eq)]
pub struct SRCKeyInterruptConfig {
    pub pwrb_h2l: bool,
    pub pwrb_l2h: bool,
    pub key0_h2l: bool,
    pub key0_l2h: bool,
    pub key1_h2l: bool,
    pub key1_l2h: bool,
    pub key2_h2l: bool,
    pub key2_l2h: bool,
    pub ac_present_h2l: bool,
    pub ac_present_l2h: bool,
    pub ec_reset_h2l: bool,
    pub ec_reset_l2h: bool,
    pub flash_wp_h2l: bool,
    pub flash_wp_l2h: bool,
}

/// configuration values that disable this feature
impl Default for SRCKeyInterruptConfig {
    fn default() -> Self {
        Self {
            pwrb_h2l: false,
            pwrb_l2h: false,
            key0_h2l: false,
            key0_l2h: false,
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
        }
    }
}

/// struct that holds the Allowed Output Pin State feauture's configuration
#[derive(Debug, PartialEq, Eq)]
pub struct SRCAllowedPinConfig {
    pub bat_disable_0: bool,
    pub bat_disable_1: bool,
    pub ec_reset_0: bool,
    pub ec_reset_1: bool,
    pub pwrb_0: bool,
    pub pwrb_1: bool,
    pub key0_0: bool,
    pub key0_1: bool,
    pub key1_0: bool,
    pub key1_1: bool,
    pub key2_0: bool,
    pub key2_1: bool,
    pub z3_wakeup_0: bool,
    pub z3_wakeup_1: bool,
    pub flash_wp_0: bool,
    pub flash_wp_1: bool,
}

/// configuration values that disable this feature
impl Default for SRCAllowedPinConfig {
    fn default() -> Self {
        Self {
            bat_disable_0: false,
            bat_disable_1: false,
            ec_reset_0: false,
            ec_reset_1: false,
            pwrb_0: false,
            pwrb_1: false,
            key0_0: false,
            key0_1: false,
            key1_0: false,
            key1_1: false,
            key2_0: false,
            key2_1: false,
            z3_wakeup_0: false,
            z3_wakeup_1: false,
            flash_wp_0: false,
            flash_wp_1: false,
        }
    }
}

/* Pin Inversion Configuration */

/// struct that holds the Pin Inversion feature's confiugration
#[derive(Debug, PartialEq, Eq)]
pub struct SRCPinInversionConfig {
    pub z3_wakeup_output: bool,
    pub lid_open_input: bool,
    pub bat_disable_output: bool,
    pub ac_present_input: bool,
    pub pwrb_output: bool,
    pub pwrb_input: bool,
    pub key0_output: bool,
    pub key0_input: bool,
    pub key1_output: bool,
    pub key1_input: bool,
    pub key2_output: bool,
    pub key2_input: bool,
}

/// configuration values that disable this feature
impl Default for SRCPinInversionConfig {
    fn default() -> Self {
        Self {
            z3_wakeup_output: false,
            lid_open_input: false,
            bat_disable_output: false,
            ac_present_input: false,
            pwrb_output: false,
            pwrb_input: false,
            key0_output: false,
            key0_input: false,
            key1_output: false,
            key1_input: false,
            key2_output: false,
            key2_input: false,
        }
    }
}

/* Auto block confiugration */

/// struct that holds the Autoblock feature's configuration
#[derive(Copy, Clone, Debug)]
pub struct SRCAutoblockConfig {
    pub pwrb_debounce_timer_us: u32,
    pub block_key0: Option<bool>,
    pub block_key1: Option<bool>,
    pub block_key2: Option<bool>,
    pub enable: bool,
}

// Default implementation for a disabled autoblock configuration with the debounce timer set to the register's reset value
impl Default for SRCAutoblockConfig {
    fn default() -> Self {
        Self {
            pwrb_debounce_timer_us: 10_000, // from register rest value
            block_key0: None,
            block_key1: None,
            block_key2: None,
            enable: false,
        }
    }
}

/* Combo Detector configuration */

/// struct that holds the Combo Detector feature's configuration
#[derive(PartialEq, Eq, Debug)]
pub struct SRCComboDetectorConfig {
    pub precondition: SRCComboDetectorPins,
    pub precondition_time_us: u32,
    pub condition: SRCComboDetectorPins,
    pub condition_time_us: u32,
    pub action: SRCComboDetectorAction,
}

// Default implementation for a disabled combo detector configuration with the timers set to the register's reset value
impl Default for SRCComboDetectorConfig {
    fn default() -> Self {
        Self {
            precondition: SRCComboDetectorPins {
                ac_present: false,
                pwrb: false,
                key0: false,
                key1: false,
                key2: false,
            },
            precondition_time_us: 0,
            condition: SRCComboDetectorPins {
                ac_present: false,
                pwrb: false,
                key0: false,
                key1: false,
                key2: false,
            },
            condition_time_us: 1,
            action: SRCComboDetectorAction {
                rst_req: false,
                ec_rst: false,
                interrupt: false,
                bat_disable: false,
            },
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct SRCComboDetectorPins {
    pub ac_present: bool,
    pub pwrb: bool,
    pub key0: bool,
    pub key1: bool,
    pub key2: bool,
}

#[derive(PartialEq, Eq, Debug)]
pub struct SRCComboDetectorAction {
    pub rst_req: bool,
    pub ec_rst: bool,
    pub interrupt: bool,
    pub bat_disable: bool,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(usize)]
pub enum SRCComboDetectorId {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
}

/* WakeUp configuration */

/// struct that hold the wakeup detection feature's configuration
#[derive(Debug, PartialEq, Eq)]
pub struct SRCWakeupConfig {
    pub ac_present_debounce_timer_us: u16,
    pub pwrb_debounce_timer_us: u16,
    pub lid_open_debounce_timer_us: u16,
    pub enabled: bool,
}

pub trait OpenTitanSysRstrClient {
    fn combo_detected(
        &self,
        input_pin_state: SRCInputPinStatus,
        combodetector_id: SRCComboDetectorId,
    );
    fn key_interrupt(&self, l2h: SRCInputPinStatus, h2l: SRCInputPinStatus);
    fn wokeup(&self, ulp_wakeup: bool);
}

pub trait OpenTitanSysRstr {
    fn get_input_state(&self) -> SRCInputPinStatus;

    /* COMBO DETECTOR */
    fn configure_combo_detector(
        &self,
        detector_id: SRCComboDetectorId,
        configuration: &SRCComboDetectorConfig,
    ) -> Result<(), ()>;
    fn get_combo_detector_configuration(
        &self,
        detector_id: SRCComboDetectorId,
    ) -> SRCComboDetectorConfig;

    fn configure_autoblock(&self, configuration: &SRCAutoblockConfig) -> Result<(), ()>;
    fn get_autoblock_confiugration(&self) -> SRCAutoblockConfig;

    fn configure_keyinterrupt(&self, configuration: &SRCKeyInterruptConfig) -> Result<(), ()>;
    fn get_keyinterrupt_confiugration(&self) -> SRCKeyInterruptConfig;

    fn configure_pin_invertion(&self, configuration: &SRCPinInversionConfig) -> Result<(), ()>;
    fn get_pin_invertion_configuration(&self) -> SRCPinInversionConfig;

    fn configure_allowed_override_pin_states(
        &self,
        configuration: &SRCAllowedPinConfig,
    ) -> Result<(), ()>;
    fn get_allowed_override_pin_state_confiugration(&self) -> SRCAllowedPinConfig;
    fn override_output_pin(&self, pin: SRCOutputPin, state: Option<bool>);

    fn configure_wakeup(&self, configuration: &SRCWakeupConfig);
    fn get_wakeup_configuration(&self) -> SRCWakeupConfig;
}
