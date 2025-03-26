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

#[derive(Clone, Copy)]
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
/// ::default() returns configuration that disables this feature
/// Each field of the format signal_edge specifies if a h2l (high to low) or l2h ( low to high) edge on the signal should trigger a key interrupt.
#[derive(Debug, PartialEq, Eq, Default)]
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

/// struct that holds the Allowed Output Pin State feauture's configuration
/// ::default() returns configuration that disables this feature
/// These fields configure which pins are allowed to be overriden, not which are actually overriden. Each signal can be allowed to be overriden to low level if the associated field ending with '_0' is set and each signal can be allowed to be overriden to high level if the associated field ending wiht '_1' is set.
#[derive(Debug, PartialEq, Eq, Default)]
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

/* Pin Inversion Configuration */

/// struct that holds the Pin Inversion feature's confiugration
/// ::default() returns configuration that disables this feature
/// Signals can be inverted at input stage (before arriving at any circuit) or output stage (before arriving at the output pins)
#[derive(Debug, PartialEq, Eq, Default)]
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
/* Auto block confiugration */

/// struct that holds the Autoblock feature's configuration
#[derive(Copy, Clone, Debug)]
pub struct SRCAutoblockConfig {
    /// the pwrb signal must exceed this debounce time by at least one clock cycle to be detected
    pub pwrb_debounce_timer_us: u32,
    /// `None` - key0 signal should not be blocked, `Some(logic_level)` - key0 signal can be blocked to logic_level
    pub block_key0: Option<bool>,
    /// `None` - key1 signal should not be blocked, `Some(logic_level)` - key1 signal can be blocked to logic_level
    pub block_key1: Option<bool>,
    /// `None` - key2 signal should not be blocked, `Some(logic_level)` - key2 signal can be blocked to logic_level
    pub block_key2: Option<bool>,
    /// should this circuit be enabled
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
/// ::default() returns configuration that disables this feature
#[derive(PartialEq, Eq, Debug, Default)]
pub struct SRCComboDetectorConfig {
    /// which pins to take into account in the first detector phase (can be disabled by configuring it as `::default()``)
    pub precondition: SRCComboDetectorPins,
    /// duration for which the combo prec-condition should pe valid in order to trigger the second stage
    pub precondition_time_us: u32,
    /// which pins to take into account in the second detector phase
    pub condition: SRCComboDetectorPins,
    /// duration for which the the the combo condition should be valid in order to trigger the action phase
    pub condition_time_us: u32,
    /// list of actions to execute when combo detector is triggered
    pub action: SRCComboDetectorAction,
}

#[derive(PartialEq, Eq, Debug, Default)]
pub struct SRCComboDetectorPins {
    pub ac_present: bool,
    pub pwrb: bool,
    pub key0: bool,
    pub key1: bool,
    pub key2: bool,
}

#[derive(PartialEq, Eq, Debug, Default)]
pub struct SRCComboDetectorAction {
    /// issue a reset request via `rst_reg_o` to the ResetManager
    pub rst_req: bool,
    /// Assert `ec_rst_l_o` for the ammount of cycles configured with `configure_pulselength_ec_rst_l_o`
    pub ec_rst: bool,
    /// issue an interrupt to the processor
    pub interrupt: bool,
    /// drive the `bat_disable` output high until the next reset
    pub bat_disable: bool,
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(usize)]
pub enum SRCComboDetectorId {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
}

impl TryFrom<u32> for SRCComboDetectorId {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            _ => Err(()),
        }
    }
}

/* WakeUp configuration */

/// struct that hold the wakeup detection feature's configuration
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SRCWakeupConfig {
    /// debounce timer value for detecting a high level (can be inverted due to input inversion functionality) on `ac_present` input signal in order to trigger a wakeup
    pub ac_present_debounce_timer_us: u16,
    /// debounce timer value for detecting a high to low transition (can be inverted due to input inversion functionality) on `pwrb` input signal in order to trigger a wakeup
    pub pwrb_debounce_timer_us: u16,
    /// debounce timer value for detecting a low to high transition (can be inverted due to input inversion functionality) on `lid_open` input signal in order to trigger a wakeup
    pub lid_open_debounce_timer_us: u16,
    /// should this feature be enabled or disabled
    pub enabled: bool,
}

pub trait OpenTitanSysRstrClient {
    /// handler called when the combo detector interrupt is triggered
    /// # Parameters
    /// * 'input_pin_state`: state of the input pins
    /// * `combodetector_id`: which combo detector was triggered
    fn combo_detected(
        &self,
        input_pin_state: SRCInputPinStatus,
        combodetector_id: SRCComboDetectorId,
    );

    /// handler called when the key interrupt interrupt is triggered
    /// # Parameters:
    /// * `l2h`: which input pins have been triggered with a low to high transition
    /// * `h2l`: which input pins have been triggered with a high to low transition
    fn key_interrupt(&self, l2h: SRCInputPinStatus, h2l: SRCInputPinStatus);

    /// handler called when it detected a wakeup condition
    /// # Parameters:
    /// * `ulp_wakeup` - set if a low power condition was triggered, unset if a normal wakeup condition was triggerd
    fn wokeup(&self, ulp_wakeup: bool);
}

pub trait OpenTitanSysRstr {
    /// read input pins state before inverstion circuits using `PIN_IN_VALUE` register
    fn get_input_state(&self) -> SRCInputPinStatus;

    /* COMBO DETECTOR */
    /// confiugre combo detector circuit
    /// # Paramters
    /// * `detector_id`  which detector to configure
    /// * `configuration` how should the circuit behave (see `SRCComboDetectorConfig` documentation for meaning of each field )
    /// # Return
    /// * `Ok`: when confiugration was successful
    /// * `Err`: when confiugration was not successful (registers locked)
    fn configure_combo_detector(
        &self,
        detector_id: SRCComboDetectorId,
        configuration: &SRCComboDetectorConfig,
    ) -> Result<(), ()>;

    /// read configuration for combo detector circuit from registers
    /// # Parameters
    /// * `detector_id`: which detector to read
    /// # Returns:
    /// * `SRCComboDetectorConfiguration` how the circuit is configured to behave (See `SRCComboDetectorConfig` documentation )
    fn get_combo_detector_configuration(
        &self,
        detector_id: SRCComboDetectorId,
    ) -> SRCComboDetectorConfig;

    /* AUTO-BLOCK key outputs */

    /// confiugre autoblock circuit
    /// # Parameters
    /// * `configuration` how should the circuit behave (see `SRCAutoblockConfig` documentation for meaning of each field)
    /// # Return
    /// * `Ok`: when confiugration was successful
    /// * `Err`: when confiugration was not successful (registers locked)
    fn configure_autoblock(&self, configuration: &SRCAutoblockConfig) -> Result<(), ()>;

    /// read configuration for combo detector circuit from registers
    /// # Parameters
    /// * `detector_id`: which detector to read
    /// # Returns:
    /// * `SRCComboDetectorConfiguration` how the circuit is configured to behave (See `SRCComboDetectorConfig` documentation )
    fn get_autoblock_confiugration(&self) -> SRCAutoblockConfig;

    /* KEY INTERRUPT */
    /// configure key interrupt functionality
    /// # Parameters
    /// * `configuration` how should the circuit behave (see `SRCKeyInterruptConfig` documentation for meaning of each field)
    /// # Return
    /// * `Ok`: when confiugration was successful
    /// * `Err`: when confiugration was not successful (registers locked)
    fn configure_keyinterrupt(&self, configuration: &SRCKeyInterruptConfig) -> Result<(), ()>;

    /// read configuration for key interrupt circuit from registers
    /// # Parameters
    /// # Returns:
    /// * `SRCKeyInterruptConfig` how the circuit is configured to behave (See `SRCComboDetectorConfig` documentation )
    fn get_keyinterrupt_confiugration(&self) -> SRCKeyInterruptConfig;

    /// configure pin inversion functionality
    /// # Parameters
    /// * `configuration` how should the circuit behave (see `SRCPinInversionConfig` documentation for meaning of each field)
    /// # Return
    /// * `Ok`: when confiugration was successful
    /// * `Err`: when confiugration was not successful (registers locked)
    fn configure_pin_invertion(&self, configuration: &SRCPinInversionConfig) -> Result<(), ()>;

    /// read configuration for pin inversion circuit from registers
    /// # Parameters
    /// # Returns:
    /// * `SRCKeyInterruptConfig` how the circuit is configured to behave (see `SRCPinInversionConfig` documentation )
    fn get_pin_invertion_configuration(&self) -> SRCPinInversionConfig;

    /// configure allowed override pin functionality
    /// # Parameters
    /// * `configuration` how should the circuit behave (see `SRCAllowedPinConfig` documentation for meaning of each field)
    /// # Return
    /// * `Ok`: when confiugration was successful
    /// * `Err`: when confiugration was not successful (registers locked)
    fn configure_allowed_override_pin_states(
        &self,
        configuration: &SRCAllowedPinConfig,
    ) -> Result<(), ()>;

    /// read configuration for allowed override circuit from registers
    /// # Parameters
    /// # Returns:
    /// * `SRCAllowedPinConfig` how the circuit is configured to behave (see `SRCAllowedPinConfig` documentation )
    fn get_allowed_override_pin_state_confiugration(&self) -> SRCAllowedPinConfig;

    /// override one of the output pins to a certain state or disable the overriding. This will work only if the pin was configured to be allowed to be overriden.
    /// # Parameters
    /// * `pin`: which output pin to override
    /// * `state: Some(logic_level)`: override pin to logic_level,
    /// * `state: None` - disable pin override
    fn override_output_pin(&self, pin: SRCOutputPin, state: Option<bool>);

    /* WAKEUP */
    /// configure this peripheral's wakeup functionality
    /// # Parameters
    /// * `configuration` how should the circuit behave (see `SRCWakeupConfig` documentation for meaning of each field)
    /// # Return
    /// * `Ok`: when configuration was successful
    /// * `Err`: when configuration was not successful (registers locked)
    fn configure_wakeup(&self, configuration: &SRCWakeupConfig);

    /// read configuration for allowed override circuit from registers
    /// # Parameters
    /// # Returns:
    /// * `SRCWakeupConfig` how the circuit is configured to behave (see `SRCWakeupConfig` documentation )
    fn get_wakeup_configuration(&self) -> SRCWakeupConfig;

    /* DEBOUNCE TIMER (shared by Key Interrupt and Combo Detectors) */
    /// configure debounce timer
    /// # Returns
    /// * `Ok`: when configuration was successful
    /// * `Err`: when configuration was not sucessful (registers locked)
    fn configure_debouncetimer(&self, duration_us: u16) -> Result<(), ()>;

    /// read duration for deboncet timer from registers. Return result in microseconds.
    fn get_debouncetimer_configuration(&self) -> u32;

    /// lock HW register from reconfiguration
    fn lock_configuration(&self);
}
