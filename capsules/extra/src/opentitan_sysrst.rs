// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Provides userspace applications with access to OpenTitan's SystemReset Controller peripheral.
//!
//! This capsule gives userspace applications access to configure and read SysRstr_Ctrl state, read input signal's state and receive callbacks when a combo detector/key interrupt/wakeup triggered.
//!//!
//! Syscall Interface
//! -----------------
//!
//! ### Commands:
//!     0 - Existence
//!     1 - Input pin state
//!     2 - Confiugre Combo Detector
//!     3 - Get Combo Detector configuration
//!     4 - Configure Key Interrupt
//!     5 - Get Key Interrupt configuration
//!     6 - Configure Autoblock
//!     7 - Get Autoblock configuration
//!     8 - Configure Pin Inversion
//!     9 - Get Pin Inversion configuration
//!    10 - Configure Allowed Override Pins
//!    11 - Get Allowed Override Pins configuration
//!    12 - Override output pin
//!    13 - Configure Wakeup
//!    14 - Get Wakeup configuration
//!    15 - Confiugre Debounce Timer (shared by Combo Detector and Key Interrupt)
//!    16 - Get debounce timer configuration
//!
//! ### Subscribes
//!
//! This capsules provides two callbacks for Combo Detector triggers and for KeyInterrupt triggers

use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};

use kernel::hil::opentitan_sysrst::{
    OpenTitanSysRstr, OpenTitanSysRstrClient, SRCAllowedPinConfig, SRCAutoblockConfig,
    SRCComboDetectorAction, SRCComboDetectorConfig, SRCComboDetectorId, SRCComboDetectorPins,
    SRCKeyInterruptConfig, SRCOutputPin, SRCPinInversionConfig, SRCWakeupConfig,
};

use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::registers::{register_bitfields, InMemoryRegister};
use kernel::{ErrorCode, ProcessId};
use OverridePin::output_pin;

/// Syscall driver number.
pub const DRIVER_NUM: usize = capsules_core::driver::NUM::OpenTitanSysRst as usize;

/// ### `subscribe_num`
///
/// - `0`: Subscribe to SysRst_Ctrl's combo detector triggers
///        The callback signature is `fn(detector_id, input_pin_state)`
/// - `1`: Subsccibe to SysRstr_Ctrl's key interrupt trigger
///        The callback signature is `fn(keys_l2h, keys_h2l)`
/// - `2`: Subscribe to SysRstr_Ctrl's wakuep trigger
///        The callback signature is fn(ulp_wakeup)
mod upcalls {
    pub const COMBO_DETECTED: usize = 0;
    pub const KEY_INTERRUPT: usize = 1;
    pub const WOKEUP: usize = 2;
    pub const COUNT: u8 = 3;
}

mod command {
    pub const EXISTENCE_CHECK: usize = 0;
    pub const INPUT_PIN_STATE: usize = 1;
    pub const CONFIGURE_COMBO_DETECTOR: usize = 2;
    pub const GET_COMBO_DETECTOR_CONFIGURATION: usize = 3;
    pub const CONFIGURE_KEYINTERRUPT: usize = 4;
    pub const GET_KEYINTERRUPT_CONFIGURATION: usize = 5;
    pub const CONFIGURE_AUTOBLOCK: usize = 6;
    pub const GET_AUTOBLOCK_CONFIGURATION: usize = 7;
    pub const CONFIGURE_PIN_INVERSION: usize = 8;
    pub const GET_PIN_INVERSION_CONFIGURATION: usize = 9;
    pub const CONFIGURE_ALLOWED_OVERRIDE_PINS: usize = 10;
    pub const GET_ALLOWED_OVERRIDE_PINS_CONFIUGRATION: usize = 11;
    pub const OVERRIDE_OUTPUT_PINS: usize = 12;
    pub const CONFIGURE_WAKEUP: usize = 13;
    pub const GET_WAKEUP_CONFIGURATION: usize = 14;
    pub const CONFIGURE_DEBOUNCETIMER: usize = 15;
    pub const GET_DEBOUNCETIMER_CONFIGURATION: usize = 16;
    pub const LOCK_CONFIGURATION: usize = 17;
}

const MASK_16_BITS: u32 = (1 << 16) - 1;

/* COMBO DETECTOR */
// functions to parse and serialize Combo Detector Configurations from/to the userspace

register_bitfields![u32,
    pub CDCompConf [
        id OFFSET(0) NUMBITS(2) [],

        precondition_ac_present OFFSET(2) NUMBITS(1) [],
        precondition_pwrb OFFSET(3) NUMBITS(1) [],
        precondition_key0 OFFSET(4) NUMBITS(1) [],
        precondition_key1 OFFSET(5) NUMBITS(1) [],
        precondition_key2 OFFSET(6) NUMBITS(1) [],

        condition_ac_present OFFSET(7) NUMBITS(1) [],
        condition_pwrb OFFSET(8) NUMBITS(1) [],
        condition_key0 OFFSET(9) NUMBITS(1) [],
        condition_key1 OFFSET(10) NUMBITS(1) [],
        condition_key2 OFFSET(11) NUMBITS(1) [],

        action_rst_req OFFSET(12) NUMBITS(1) [],
        action_ec_rst OFFSET(13) NUMBITS(1) [],
        action_interrupt OFFSET(14) NUMBITS(1) [],
        action_bat_disable OFFSET(15) NUMBITS(1) [],

    ],
];

fn parse_combodetector_configuration(
    input1: u32,
    input2: u32,
) -> Option<(SRCComboDetectorId, SRCComboDetectorConfig)> {
    let first_field = InMemoryRegister::<u32, CDCompConf::Register>::new(input1);
    let id = SRCComboDetectorId::try_from(0).ok()?;

    // deserialize `input2` that contains the `precondition_time` and `condition_time`
    let precondition_time_raw = input2 & MASK_16_BITS;
    let condition_time_raw = (input2 >> 16) & MASK_16_BITS;

    // scale the two values as the HW has a 5us resolution
    let precondition_time_us = precondition_time_raw * 5;
    let condition_time_us = condition_time_raw * 5;

    let config = SRCComboDetectorConfig {
        precondition: SRCComboDetectorPins {
            ac_present: first_field.read(CDCompConf::precondition_ac_present) != 0,
            pwrb: first_field.read(CDCompConf::precondition_pwrb) != 0,
            key0: first_field.read(CDCompConf::precondition_key0) != 0,
            key1: first_field.read(CDCompConf::precondition_key1) != 0,
            key2: first_field.read(CDCompConf::precondition_key2) != 0,
        },
        precondition_time_us,
        condition: SRCComboDetectorPins {
            ac_present: first_field.read(CDCompConf::condition_ac_present) != 0,
            pwrb: first_field.read(CDCompConf::condition_pwrb) != 0,
            key0: first_field.read(CDCompConf::condition_key0) != 0,
            key1: first_field.read(CDCompConf::condition_key1) != 0,
            key2: first_field.read(CDCompConf::condition_key2) != 0,
        },
        condition_time_us: condition_time_us,
        action: SRCComboDetectorAction {
            rst_req: first_field.read(CDCompConf::action_rst_req) != 0,
            ec_rst: first_field.read(CDCompConf::action_ec_rst) != 0,
            interrupt: first_field.read(CDCompConf::action_interrupt) != 0,
            bat_disable: first_field.read(CDCompConf::action_bat_disable) != 0,
        },
    };

    Some((id, config))
}

fn serialize_combodetector_configuration(
    config: &SRCComboDetectorConfig,
    id: &SRCComboDetectorId,
) -> (u32, u32) {
    // encode `id`, `precondition`, `condition`, `action` fields from configuration into the first u32 bitfield
    let data1 = InMemoryRegister::<u32, CDCompConf::Register>::new(0);
    data1.write(
        CDCompConf::id.val(*id as u32)
            + CDCompConf::precondition_ac_present.val(config.precondition.ac_present as u32)
            + CDCompConf::precondition_pwrb.val(config.precondition.pwrb as u32)
            + CDCompConf::precondition_key0.val(config.precondition.key0 as u32)
            + CDCompConf::precondition_key1.val(config.precondition.key1 as u32)
            + CDCompConf::precondition_key2.val(config.precondition.key2 as u32)
            + CDCompConf::condition_ac_present.val(config.condition.ac_present as u32)
            + CDCompConf::condition_pwrb.val(config.condition.pwrb as u32)
            + CDCompConf::condition_key0.val(config.condition.key0 as u32)
            + CDCompConf::condition_key1.val(config.condition.key1 as u32)
            + CDCompConf::condition_key2.val(config.condition.key2 as u32)
            + CDCompConf::action_rst_req.val(config.action.rst_req as u32)
            + CDCompConf::action_ec_rst.val(config.action.ec_rst as u32)
            + CDCompConf::action_interrupt.val(config.action.interrupt as u32)
            + CDCompConf::action_bat_disable.val(config.action.bat_disable as u32),
    );

    // encode `precondition_time_us` and `condition_time_us` from configuration into the second u32 field
    let precondition_time_encoded =
        u32::div_ceil(config.precondition_time_us, 5).min(u16::MAX as u32);
    let condition_time_encoded = u32::div_ceil(config.condition_time_us, 5).min(u16::MAX as u32);
    let data2 = precondition_time_encoded + (condition_time_encoded << 16);

    (data1.get(), data2)
}

/* KEY INTERRUPT */

// functions to parse and serialize Key Interrupt Configurations from/to the userspace
register_bitfields![u32,
    pub KeyInterruptState [
        pwrb_h2l 0,
        key0_in_h2l 1,
        key1_in_h2l 2,
        key2_in_h2l 3,
        ac_present_h2l 4,
        ec_rst_h2l 5,
        flash_wp_h2l 6,
        pwrb_l2h 7,
        key0_in_l2h 8,
        key1_in_l2h 9,
        key2_in_l2h 10,
        ac_present_l2h 11,
        ec_rst_l2h 12,
        flash_wp_l2h 13,
    ]
];

fn parse_keyinterrupt(data1: usize) -> Option<SRCKeyInterruptConfig> {
    let first_field = InMemoryRegister::<u32, KeyInterruptState::Register>::new(data1 as u32);

    // check that data1 doesn't contain other data after the first 14 bits (see `KeyInterruptState`)
    if data1 >= (1 << 14) {
        return None;
    }
    Some(SRCKeyInterruptConfig {
        pwrb_h2l: first_field.read(KeyInterruptState::pwrb_h2l) != 0,
        pwrb_l2h: first_field.read(KeyInterruptState::pwrb_l2h) != 0,
        key0_h2l: first_field.read(KeyInterruptState::key0_in_h2l) != 0,
        key0_l2h: first_field.read(KeyInterruptState::key0_in_l2h) != 0,
        key1_h2l: first_field.read(KeyInterruptState::key1_in_h2l) != 0,
        key1_l2h: first_field.read(KeyInterruptState::key1_in_l2h) != 0,
        key2_h2l: first_field.read(KeyInterruptState::key2_in_h2l) != 0,
        key2_l2h: first_field.read(KeyInterruptState::key2_in_l2h) != 0,
        ac_present_h2l: first_field.read(KeyInterruptState::ac_present_h2l) != 0,
        ac_present_l2h: first_field.read(KeyInterruptState::ac_present_l2h) != 0,
        ec_reset_h2l: first_field.read(KeyInterruptState::ec_rst_h2l) != 0,
        ec_reset_l2h: first_field.read(KeyInterruptState::ec_rst_l2h) != 0,
        flash_wp_h2l: first_field.read(KeyInterruptState::flash_wp_h2l) != 0,
        flash_wp_l2h: first_field.read(KeyInterruptState::flash_wp_l2h) != 0,
    })
}

fn serialize_key_interrupt_configuration(config: SRCKeyInterruptConfig) -> u32 {
    let data1 = InMemoryRegister::<u32, KeyInterruptState::Register>::new(0);
    data1.write(
        KeyInterruptState::pwrb_h2l.val(config.pwrb_h2l as u32)
            + KeyInterruptState::pwrb_l2h.val(config.pwrb_l2h as u32)
            + KeyInterruptState::key0_in_h2l.val(config.key0_h2l as u32)
            + KeyInterruptState::key0_in_l2h.val(config.key0_l2h as u32)
            + KeyInterruptState::key1_in_h2l.val(config.key1_h2l as u32)
            + KeyInterruptState::key1_in_l2h.val(config.key1_l2h as u32)
            + KeyInterruptState::key2_in_h2l.val(config.key2_h2l as u32)
            + KeyInterruptState::key2_in_l2h.val(config.key2_l2h as u32)
            + KeyInterruptState::ac_present_h2l.val(config.ac_present_h2l as u32)
            + KeyInterruptState::ac_present_l2h.val(config.ac_present_l2h as u32)
            + KeyInterruptState::ec_rst_h2l.val(config.ec_reset_h2l as u32)
            + KeyInterruptState::ec_rst_l2h.val(config.ec_reset_l2h as u32)
            + KeyInterruptState::flash_wp_h2l.val(config.flash_wp_h2l as u32)
            + KeyInterruptState::flash_wp_l2h.val(config.flash_wp_l2h as u32),
    );

    data1.get()
}

/* AUTO BLOCK */
// functions to parse and serialize Auto Block Configurations from/to the userspace

register_bitfields![u32,
    pub AutoblockConfiguration [
        debounce_timer OFFSET(0) NUMBITS(16),
        block_key0 OFFSET(16) NUMBITS(1),
        state_key0 OFFSET(17) NUMBITS(1),
        block_key1 OFFSET(18) NUMBITS(1),
        state_key1 OFFSET(19) NUMBITS(1),
        block_key2 OFFSET(20) NUMBITS(1),
        state_key2 OFFSET(21) NUMBITS(1),
        enable OFFSET(22) NUMBITS(1),
    ]
];

fn parse_autoblock_configuration(data1: usize) -> Option<SRCAutoblockConfig> {
    let first_field = InMemoryRegister::<u32, AutoblockConfiguration::Register>::new(data1 as u32);

    let block_key0 = first_field.is_set(AutoblockConfiguration::block_key0);
    let state_key0 = first_field.is_set(AutoblockConfiguration::state_key0);

    let block_key1 = first_field.is_set(AutoblockConfiguration::block_key1);
    let state_key1 = first_field.is_set(AutoblockConfiguration::state_key1);

    let block_key2 = first_field.is_set(AutoblockConfiguration::block_key2);
    let state_key2 = first_field.is_set(AutoblockConfiguration::state_key2);

    // check that configuration bitfield doesn't contain extra bits
    if data1 >= (1 << 23) {
        return None;
    }
    Some(SRCAutoblockConfig {
        pwrb_debounce_timer_us: first_field.read(AutoblockConfiguration::debounce_timer),
        block_key0: block_key0.then_some(state_key0),
        block_key1: block_key1.then_some(state_key1),
        block_key2: block_key2.then_some(state_key2),
        enable: first_field.is_set(AutoblockConfiguration::enable),
    })
}

fn serialize_autoblock_configuration(config: SRCAutoblockConfig) -> u32 {
    let data1 = InMemoryRegister::<u32, AutoblockConfiguration::Register>::new(0);
    data1.write(
        AutoblockConfiguration::debounce_timer.val(config.pwrb_debounce_timer_us)
            + AutoblockConfiguration::block_key0.val(config.block_key0.is_some() as u32)
            + AutoblockConfiguration::state_key0.val(config.block_key0.unwrap_or(false) as u32)
            + AutoblockConfiguration::block_key1.val(config.block_key1.is_some() as u32)
            + AutoblockConfiguration::state_key1.val(config.block_key1.unwrap_or(false) as u32)
            + AutoblockConfiguration::block_key2.val(config.block_key2.is_some() as u32)
            + AutoblockConfiguration::state_key2.val(config.block_key2.unwrap_or(false) as u32)
            + AutoblockConfiguration::enable.val(config.enable as u32),
    );
    data1.get()
}

/* PIN INVERSION */
// functions to parse and serialize Pin Inversion Configurations from/to the userspace

register_bitfields![u32,
    pub PinInversionConfig [
        z3_wakeup_output 0,
        lid_open_input 1,
        bat_disable_output 2,
        ac_present_input 3,
        pwrb_output 4,
        pwrb_input 5,
        key0_output 6,
        key0_input 7,
        key1_output 8,
        key1_input 9,
        key2_output 10,
        key2_input 11,
    ]
];

fn parse_pin_inversion_configuration(data1: usize) -> Option<SRCPinInversionConfig> {
    let first_field = InMemoryRegister::<u32, PinInversionConfig::Register>::new(data1 as u32);

    if data1 >= (1 << 12) {
        return None;
    }
    Some(SRCPinInversionConfig {
        z3_wakeup_output: first_field.is_set(PinInversionConfig::z3_wakeup_output),
        lid_open_input: first_field.is_set(PinInversionConfig::lid_open_input),
        bat_disable_output: first_field.is_set(PinInversionConfig::bat_disable_output),
        ac_present_input: first_field.is_set(PinInversionConfig::ac_present_input),
        pwrb_output: first_field.is_set(PinInversionConfig::pwrb_output),
        pwrb_input: first_field.is_set(PinInversionConfig::pwrb_input),
        key0_output: first_field.is_set(PinInversionConfig::key0_output),
        key0_input: first_field.is_set(PinInversionConfig::key0_input),
        key1_output: first_field.is_set(PinInversionConfig::key1_output),
        key1_input: first_field.is_set(PinInversionConfig::key1_input),
        key2_output: first_field.is_set(PinInversionConfig::key2_output),
        key2_input: first_field.is_set(PinInversionConfig::key2_input),
    })
}

fn serialize_pin_inversion_configuration(config: SRCPinInversionConfig) -> u32 {
    let data1 = InMemoryRegister::<u32, PinInversionConfig::Register>::new(0);
    data1.write(
        PinInversionConfig::z3_wakeup_output.val(config.z3_wakeup_output as u32)
            + PinInversionConfig::bat_disable_output.val(config.bat_disable_output as u32)
            + PinInversionConfig::ac_present_input.val(config.ac_present_input as u32)
            + PinInversionConfig::pwrb_output.val(config.pwrb_output as u32)
            + PinInversionConfig::pwrb_input.val(config.pwrb_input as u32)
            + PinInversionConfig::key0_output.val(config.key0_output as u32)
            + PinInversionConfig::key0_input.val(config.key0_input as u32)
            + PinInversionConfig::key1_output.val(config.key1_output as u32)
            + PinInversionConfig::key1_input.val(config.key1_input as u32)
            + PinInversionConfig::key2_output.val(config.key2_output as u32)
            + PinInversionConfig::key2_input.val(config.key2_input as u32),
    );
    data1.get()
}

/* ALLOWED OVERRIDE */

// functions to parse and serialize Allowed Override Configurations from/to the userspace
register_bitfields![u32,
    pub AllowedOverridePinConfig [
        bat_disable_0 0,
        bat_disable_1 1,
        ec_reset_0 2,
        ec_reset_1 3,
        pwrb_0 4,
        pwrb_1 5,
        key0_0 6,
        key0_1 7,
        key1_0 8,
        key1_1 9,
        key2_0 10,
        key2_1 11,
        z3_wakeup_0 12,
        z3_wakeup_1 13,
        flash_wp_0 14,
        flash_wp_1 15,
    ]
];

fn parse_allowedoverridepin_configuration(data1: usize) -> Option<SRCAllowedPinConfig> {
    let first_field =
        InMemoryRegister::<u32, AllowedOverridePinConfig::Register>::new(data1 as u32);

    if data1 >= (1 << 16) {
        return None;
    }
    Some(SRCAllowedPinConfig {
        bat_disable_0: first_field.is_set(AllowedOverridePinConfig::bat_disable_0),
        bat_disable_1: first_field.is_set(AllowedOverridePinConfig::bat_disable_1),
        ec_reset_0: first_field.is_set(AllowedOverridePinConfig::ec_reset_0),
        ec_reset_1: first_field.is_set(AllowedOverridePinConfig::ec_reset_1),
        pwrb_0: first_field.is_set(AllowedOverridePinConfig::pwrb_0),
        pwrb_1: first_field.is_set(AllowedOverridePinConfig::pwrb_1),
        key0_0: first_field.is_set(AllowedOverridePinConfig::key0_0),
        key0_1: first_field.is_set(AllowedOverridePinConfig::key0_1),
        key1_0: first_field.is_set(AllowedOverridePinConfig::key1_0),
        key1_1: first_field.is_set(AllowedOverridePinConfig::key1_1),
        key2_0: first_field.is_set(AllowedOverridePinConfig::key2_0),
        key2_1: first_field.is_set(AllowedOverridePinConfig::key2_1),
        z3_wakeup_0: first_field.is_set(AllowedOverridePinConfig::z3_wakeup_0),
        z3_wakeup_1: first_field.is_set(AllowedOverridePinConfig::z3_wakeup_1),
        flash_wp_0: first_field.is_set(AllowedOverridePinConfig::flash_wp_0),
        flash_wp_1: first_field.is_set(AllowedOverridePinConfig::flash_wp_1),
    })
}

fn serialize_allowedoverridepin_configuration(config: SRCAllowedPinConfig) -> u32 {
    let data1 = InMemoryRegister::<u32, AllowedOverridePinConfig::Register>::new(0);
    data1.write(
        AllowedOverridePinConfig::bat_disable_0.val(config.bat_disable_0 as u32)
            + AllowedOverridePinConfig::bat_disable_1.val(config.bat_disable_1 as u32)
            + AllowedOverridePinConfig::ec_reset_0.val(config.ec_reset_0 as u32)
            + AllowedOverridePinConfig::ec_reset_1.val(config.ec_reset_1 as u32)
            + AllowedOverridePinConfig::pwrb_0.val(config.pwrb_0 as u32)
            + AllowedOverridePinConfig::pwrb_1.val(config.pwrb_1 as u32)
            + AllowedOverridePinConfig::key0_0.val(config.key0_0 as u32)
            + AllowedOverridePinConfig::key0_1.val(config.key0_1 as u32)
            + AllowedOverridePinConfig::key1_0.val(config.key1_0 as u32)
            + AllowedOverridePinConfig::key1_1.val(config.key1_1 as u32)
            + AllowedOverridePinConfig::key2_0.val(config.key2_0 as u32)
            + AllowedOverridePinConfig::key2_1.val(config.key2_1 as u32)
            + AllowedOverridePinConfig::z3_wakeup_0.val(config.z3_wakeup_0 as u32)
            + AllowedOverridePinConfig::z3_wakeup_1.val(config.z3_wakeup_1 as u32)
            + AllowedOverridePinConfig::flash_wp_0.val(config.flash_wp_0 as u32)
            + AllowedOverridePinConfig::flash_wp_1.val(config.flash_wp_1 as u32),
    );
    data1.get()
}

/* OVERRIDE PIN */

// functions to parse and serialize Override Pin Configurations from/to the userspace
register_bitfields![u32,
    pub OverridePin [
        output_pin OFFSET(0) NUMBITS(3) [
            BatDisable = 0,
            EcReset = 1,
            Pwrb = 2,
            Key0 = 3,
            Key1 = 4,
            Key2 = 5,
            Z3Wakeup = 6,
            FlashWP = 7,
        ],
        state OFFSET(4) NUMBITS(2) [
            NoOverride = 0,
            OverrideHigh = 1,
            OverrideLow = 2,
        ],
    ]
];

fn parse_overridepin(data1: usize) -> Option<(SRCOutputPin, Option<bool>)> {
    let first_field = InMemoryRegister::<u32, OverridePin::Register>::new(data1 as u32);

    let out_pin: SRCOutputPin = match first_field
        .read_as_enum::<OverridePin::output_pin::Value>(OverridePin::output_pin)?
    {
        output_pin::Value::BatDisable => SRCOutputPin::BatDisable,
        output_pin::Value::EcReset => SRCOutputPin::EcReset,
        output_pin::Value::Pwrb => SRCOutputPin::Pwrb,
        output_pin::Value::Key0 => SRCOutputPin::Key0,
        output_pin::Value::Key1 => SRCOutputPin::Key1,
        output_pin::Value::Key2 => SRCOutputPin::Key2,
        output_pin::Value::Z3Wakeup => SRCOutputPin::Z3Wakeup,
        output_pin::Value::FlashWP => SRCOutputPin::FlashWP,
    };

    let state = match first_field.read_as_enum::<OverridePin::state::Value>(OverridePin::state)? {
        OverridePin::state::Value::NoOverride => None,
        OverridePin::state::Value::OverrideHigh => Some(true),
        OverridePin::state::Value::OverrideLow => Some(false),
    };

    Some((out_pin, state))
}

/* WAKEUP */

// functions to parse and serialize Wakeup Configurations from/to the userspace
fn parse_wakeupconfig(data1: u32, data2: u32) -> Option<SRCWakeupConfig> {
    let ac_present_debounce_timer_us = (data1 & MASK_16_BITS) as u16;
    let pwrb_debounce_timer_us = (data1 >> 16) as u16;
    let lid_open_debounce_timer_us = (data2 & MASK_16_BITS) as u16;
    let enabled = (data2 & (1 << 16)) != 0;

    if data2 >= (1 << 17) {
        return None;
    }

    Some(SRCWakeupConfig {
        ac_present_debounce_timer_us,
        pwrb_debounce_timer_us,
        lid_open_debounce_timer_us,
        enabled,
    })
}

fn serialize_wakeupconfig(config: SRCWakeupConfig) -> (u32, u32) {
    let data1 =
        ((config.pwrb_debounce_timer_us as u32) << 16) + config.ac_present_debounce_timer_us as u32;
    let data2 = ((config.enabled as u32) << 16) + config.lid_open_debounce_timer_us as u32;

    (data1, data2)
}
pub struct SystemReset<'a, Driver: OpenTitanSysRstr> {
    driver: &'a Driver,
    grants: Grant<(), UpcallCount<{ upcalls::COUNT }>, AllowRoCount<0>, AllowRwCount<0>>,
    owning_process: OptionalCell<ProcessId>,
}

impl<'a, Driver: OpenTitanSysRstr> SystemReset<'a, Driver> {
    pub fn new(
        driver: &'a Driver,
        grant: Grant<(), UpcallCount<{ upcalls::COUNT }>, AllowRoCount<0>, AllowRwCount<0>>,
    ) -> Self {
        Self {
            driver: driver,
            grants: grant,
            owning_process: OptionalCell::empty(),
        }
    }

    /* COMBO DETECTOR */

    /// parse data received from userspace and configure combo detector section of HW/driver
    /// # Inputs:
    ///     * data1: bitfield according to `CDCompConf` (id, precondition, condition, action)
    ///     * data2: [16bits: condition_time_us][16bits: precondition_time_us]
    /// # Returns:
    ///     * Ok(()) - parse and configuration done
    ///     * Err( ErrorCode::INVAL) - parse failed
    ///     * Err( ErrorCode::FAIL) - driver configuration failed
    fn configure_combo_detector(&self, data1: usize, data2: usize) -> Result<(), ErrorCode> {
        let parsed_configuration = parse_combodetector_configuration(data1 as u32, data2 as u32)
            .ok_or(ErrorCode::INVAL)?;

        self.driver
            .configure_combo_detector(parsed_configuration.0, &parsed_configuration.1)
            .or(Err(ErrorCode::FAIL))
    }

    /// retrieve combo detector configuration from driver and serialize it for userspace
    // # Inputs
    //     * data1: combo detector id (<=3)
    // # Returns:
    //     * OK( data1, data2) with:
    //         data1: bitfield according to `CDCompConf` (id, precondition, condition, action)
    //         data2: [16bits: condition_time_us][16bits: precondition_time_us]
    //     * Err ( ErrorCode::INVAL) when the input `id` is invalid (>=4)
    fn get_combo_detector_configuration(&self, data1: usize) -> Result<(u32, u32), ErrorCode> {
        let detector_id =
            SRCComboDetectorId::try_from(data1 as u32).map_err(|()| ErrorCode::INVAL)?;
        let config = self.driver.get_combo_detector_configuration(detector_id);

        let a = serialize_combodetector_configuration(&config, &detector_id);
        Ok(a)
    }

    /* KEY INTERRUPT */

    /// parse data received from userspace and configure key interrupt section of HW/driver
    /// # Inputs:
    ///     * data1: bitfield according to `KeyInterruptState` (each pin has to flags for h2l and l2h transitions)
    /// # Returns:
    ///     * Ok(()) - parse and configuration done
    ///     * Err( ErrorCode::INVAL) - parse failed
    ///     * Err( ErrorCode::FAIL) - driver configuration failed
    fn configure_keyinterrupt(&self, data1: usize) -> Result<(), ErrorCode> {
        let parsed_configuration = parse_keyinterrupt(data1).ok_or(ErrorCode::INVAL)?;

        self.driver
            .configure_keyinterrupt(&parsed_configuration)
            .or(Err(ErrorCode::FAIL))
    }

    /// retrieve key interrupt configuration from driver and serialize it for userspace
    // # Returns:
    // * data: bitfield according to `KeyInterruptState` (h2l and l2h flag for each pin)
    fn get_keyinterrupt_configuration(&self) -> u32 {
        let config = self.driver.get_keyinterrupt_confiugration();
        serialize_key_interrupt_configuration(config)
    }

    /* AUTO BLOCK */
    /// parse data received from userspace and configure auto block section of HW/driver
    /// # Inputs:
    ///     * data1: bitfield according to `KeyInterruptState` (each pin has to flags for h2l and l2h transitions)
    /// # Returns:
    ///     * Ok(()) - parse and configuration done
    ///     * Err( ErrorCode::INVAL) - parse failed
    ///     * Err( ErrorCode::FAIL) - driver configuration failed
    fn configure_autoblock(&self, data1: usize) -> Result<(), ErrorCode> {
        let parsed_configuration = parse_autoblock_configuration(data1).ok_or(ErrorCode::INVAL)?;

        self.driver
            .configure_autoblock(&parsed_configuration)
            .map_err(|()| ErrorCode::FAIL)
    }

    /// retrieve autoblock configuration from driver and serialize it for userspace
    // # Returns:
    // * data: bitfield according to `AutoblockConfiguration` (debounce_timer, block_keyX, state_keyX, enable)
    fn get_autoblock_confiugration(&self) -> u32 {
        let config = self.driver.get_autoblock_confiugration();
        serialize_autoblock_configuration(config)
    }

    /* PIN INVERSION */

    /// parse data received from userspace and configure pin inversion section of HW/driver
    /// # Inputs:
    ///     * data1: bitfield according to `PinInversionConfig`
    /// # Returns:
    ///     * Ok(()) - parse and configuration done
    ///     * Err( ErrorCode::INVAL) - parse failed
    ///     * Err( ErrorCode::FAIL) - driver configuration failed
    fn configure_pin_invertion_configuration(&self, data1: usize) -> Result<(), ErrorCode> {
        let parsed_configuration =
            parse_pin_inversion_configuration(data1).ok_or(ErrorCode::INVAL)?;

        self.driver
            .configure_pin_invertion(&parsed_configuration)
            .map_err(|()| ErrorCode::FAIL)
    }

    /// retrieve pin inversion configuration from driver and serialize it for userspace
    // # Returns:
    // * data: bitfield according to `PinInversionConfig` (input stage pin invert, output stage pin invert for each pin)
    fn get_pin_invertion_configuration(&self) -> u32 {
        let config = self.driver.get_pin_invertion_configuration();
        serialize_pin_inversion_configuration(config)
    }

    /// parse data received from userspace and configure auto block section of HW/driver
    /// # Inputs:
    ///     * data1: bitfield according to `KeyInterruptState` (each pin has to flags for h2l and l2h transitions)
    /// # Returns:
    ///     * Ok(()) - parse and configuration done
    ///     * Err( ErrorCode::INVAL) - parse failed
    ///     * Err( ErrorCode::FAIL) - driver configuration failed
    fn configure_allowed_override_pin_config(&self, data1: usize) -> Result<(), ErrorCode> {
        let parsed_configuration =
            parse_allowedoverridepin_configuration(data1).ok_or(ErrorCode::INVAL)?;

        self.driver
            .configure_allowed_override_pin_states(&parsed_configuration)
            .map_err(|()| ErrorCode::FAIL)
    }

    /// retrieve pin inversion configuration from driver and serialize it for userspace
    // # Returns:
    // * data: bitfield according to `AllowedOverridePinConfig` (allowed oveeride to low/high for each pin)
    fn get_allowed_override_pin_state_confiugration(&self) -> u32 {
        let config = self.driver.get_allowed_override_pin_state_confiugration();
        serialize_allowedoverridepin_configuration(config)
    }

    /// parse data received from userspace and override pin state
    /// # Inputs:
    ///     * data1: bitfield according to `OverridePin` (pin id, override state (no override, override to high/ to low))
    /// # Returns:
    ///     * Ok(()) - parse and configuration done
    ///     * Err( ErrorCode::INVAL) - parse failed
    fn override_output_pin(&self, data1: usize) -> Result<(), ErrorCode> {
        let (parsed_pin, parsed_state) = parse_overridepin(data1).ok_or(ErrorCode::INVAL)?;

        self.driver.override_output_pin(parsed_pin, parsed_state);
        Ok(())
    }

    /// parse data received from userspace and configure wakeup section of HW/driver
    /// # Inputs:
    ///     * data1: bitfield with debounce timer for ac_present and for pwrb inputs
    ///     * data2: bitfiedl with debounce timer for lid_open and enable flag
    /// # Returns:
    ///     * Ok(()) - parse and configuration done
    ///     * Err( ErrorCode::INVAL) - parse failed
    fn configure_wakeup(&self, data1: usize, data2: usize) -> Result<(), ErrorCode> {
        let parsed_configuration =
            parse_wakeupconfig(data1 as u32, data2 as u32).ok_or(ErrorCode::INVAL)?;

        self.driver.configure_wakeup(&parsed_configuration);
        Ok(())
    }

    /// retrieve pin inversion configuration from driver and serialize it for userspace
    // # Returns:
    // * (data1, data2) with:
    //      * data1: bitfield with debounce timer for ac_present and for pwrb inputs
    //      * data2: bitfield with debounce timer for lid_open and enable flag
    fn get_wakeup_configuration(&self) -> (u32, u32) {
        let config = self.driver.get_wakeup_configuration();

        let (data1, data2) = serialize_wakeupconfig(config);

        (data1, data2)
    }
}

impl<'a, Driver: OpenTitanSysRstr> SyscallDriver for SystemReset<'a, Driver> {
    fn command(
        &self,
        command_num: usize,
        data1: usize,
        data2: usize,
        calling_process: ProcessId,
    ) -> CommandReturn {
        // Check existence (regardless of which process2 asks)
        if command_num == command::EXISTENCE_CHECK {
            return CommandReturn::success();
        }

        // determine if `owning_process` is set and it exists
        // determine if the owning process matches the calling process
        let same_procees_or_empty = self.owning_process.map_or(None, |current_process| {
            self.grants
                .enter(current_process, |_, _| current_process == calling_process)
                .ok()
        });

        match same_procees_or_empty {
            // the `calling_process` and the `owning_process` are not the same
            Some(false) => return CommandReturn::failure(ErrorCode::RESERVE),
            // the `owning_process` isn't set/doesn't exist, continue execution
            None => self.owning_process.set(calling_process),
            // the  owning process` and the `calling_process` are the same, continue execution
            Some(true) => {}
        }

        match command_num {
            // get input pins state
            // will always return a bitfield with the input pin state according to `SRCInputPinState`
            command::INPUT_PIN_STATE => {
                let pin_state = self.driver.get_input_state();
                CommandReturn::success_u32(pin_state.get())
            }

            // configure one of the combo detectors
            // #Inputs:
            //    * data1: bitfield according to `CDCompConf` (id, precondition, condition, action)
            //    * data2: [16bits: condition_time_us][16bits: precondition_time_us]
            // # Returns:
            //    * Success if configuration went successfully
            //    * Failure ( ErrorCode::INVAL) when the input `id` is invalid (>=4)
            //    * Failure ( ErrorCode::FAIL) when the driver could not configure the combo detector (most likely the HW registers are locked)
            command::CONFIGURE_COMBO_DETECTOR => {
                let action_result = self.configure_combo_detector(data1, data2);
                CommandReturn::from(action_result)
            }

            // retrieve the configuration of one of the combo detectors from HW registers
            // # Inputs
            //     * data1: combo detector id (<=3)
            // # Returns:
            //     * Success_u32_u32( data1, data2) with:
            //         data1: bitfield according to `CDCompConf` (id, precondition, condition, action)
            //         data2: [16bits: condition_time_us][16bits: precondition_time_us]
            //     * Failure ( ErrorCode::INVAL) when the input `id` is invalid (>=4)
            command::GET_COMBO_DETECTOR_CONFIGURATION => {
                let action_result = self.get_combo_detector_configuration(data1);
                CommandReturn::from(action_result)
            }
            command::CONFIGURE_KEYINTERRUPT => {
                let action_result = self.configure_keyinterrupt(data1);
                CommandReturn::from(action_result)
            }
            command::GET_KEYINTERRUPT_CONFIGURATION => {
                CommandReturn::success_u32(self.get_keyinterrupt_configuration())
            }
            command::CONFIGURE_AUTOBLOCK => {
                let action_result = self.configure_autoblock(data1);
                CommandReturn::from(action_result)
            }
            command::GET_AUTOBLOCK_CONFIGURATION => {
                CommandReturn::success_u32(self.get_autoblock_confiugration())
            }
            command::CONFIGURE_PIN_INVERSION => {
                let action_result = self.configure_pin_invertion_configuration(data1);
                CommandReturn::from(action_result)
            }
            command::GET_PIN_INVERSION_CONFIGURATION => {
                CommandReturn::success_u32(self.get_pin_invertion_configuration())
            }
            command::CONFIGURE_ALLOWED_OVERRIDE_PINS => {
                let action_result = self.configure_allowed_override_pin_config(data1);
                CommandReturn::from(action_result)
            }
            command::GET_ALLOWED_OVERRIDE_PINS_CONFIUGRATION => {
                CommandReturn::success_u32(self.get_allowed_override_pin_state_confiugration())
            }
            command::OVERRIDE_OUTPUT_PINS => {
                let action_result = self.override_output_pin(data1);
                CommandReturn::from(action_result)
            }
            command::CONFIGURE_WAKEUP => {
                let action_result = self.configure_wakeup(data1, data2);
                CommandReturn::from(action_result)
            }
            command::GET_WAKEUP_CONFIGURATION => {
                let result = self.get_wakeup_configuration();
                CommandReturn::success_u32_u32(result.0, result.1)
            }
            command::CONFIGURE_DEBOUNCETIMER => CommandReturn::from(
                self.driver
                    .configure_debouncetimer(data1 as u16)
                    .map_err(|()| ErrorCode::FAIL),
            ),
            command::GET_DEBOUNCETIMER_CONFIGURATION => {
                CommandReturn::success_u32(self.driver.get_debouncetimer_configuration())
            }
            command::LOCK_CONFIGURATION => {
                self.driver.lock_configuration();
                CommandReturn::success()
            }
            _ => CommandReturn::failure(ErrorCode::NOSUPPORT),
        }
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
        self.grants.enter(processid, |_, _| {})
    }
}

impl<'a, Driver: OpenTitanSysRstr> OpenTitanSysRstrClient for SystemReset<'a, Driver> {
    fn combo_detected(
        &self,
        input_pin_state: kernel::hil::opentitan_sysrst::SRCInputPinStatus,
        combodetector_id: kernel::hil::opentitan_sysrst::SRCComboDetectorId,
    ) {
        // schedule a COMBO_DETECTED upcall with
        // * r0 = combo detector id
        // * r1 = input pin state in the order defined by `SRCInputPinState`
        // * r2 = 0
        let result = self.owning_process.map(|pid| {
            self.grants.enter(pid, |_app, upcalls| {
                upcalls.schedule_upcall(
                    upcalls::COMBO_DETECTED,
                    (combodetector_id as usize, input_pin_state.get() as usize, 0),
                )
            })
        });

        // no error handling, upcall scheduling will not be retried if an issue appears
        match result {
            // when the upcall was successful
            Some(Ok(Ok(()))) => {}
            // if the upcall coudln't be made (the owning process is registered) (`.schedule upcall` failed)
            Some(Ok(Err(_err))) => {}
            // if the grant is not available (`.enter` failed)
            Some(Err(_err)) => {}
            // if the owning process is not registered
            None => {}
        }
    }

    fn key_interrupt(
        &self,
        l2h: kernel::hil::opentitan_sysrst::SRCInputPinStatus,
        h2l: kernel::hil::opentitan_sysrst::SRCInputPinStatus,
    ) {
        // schedule a KEY_INTERRUPT upcall with
        // * r0 = keys where a L2H transition was detected
        // * r1 = keys where a H2L transition was detected
        // * r2 = 0
        let result = self.owning_process.map(|pid| {
            self.grants.enter(pid, |_app, upcalls| {
                upcalls.schedule_upcall(
                    upcalls::KEY_INTERRUPT,
                    (l2h.get() as usize, h2l.get() as usize, 0),
                )
            })
        });

        // no error handling, upcall scheduling will not be retried if an issue appears
        match result {
            // when the upcall was successful
            Some(Ok(Ok(()))) => {}
            // if the upcall coudln't be made (the owning process is registered) (`.schedule upcall`` failed)
            Some(Ok(Err(_err))) => {}
            // if the grant is not available (`.enter` failed)
            Some(Err(_err)) => {}
            // if the owning process is not registered
            None => {}
        }
    }

    fn wokeup(&self, ulp_wakeup: bool) {
        // schedule a WAKEUP upcall with
        // * r0 = did a ulp_wakeup happen,
        // * r1, r2 = 0
        let result = self.owning_process.map(|pid| {
            self.grants.enter(pid, |_app, upcalls| {
                upcalls.schedule_upcall(upcalls::WOKEUP, (ulp_wakeup as usize, 0, 0))
            })
        });

        // no error handling, upcall scheduling will not be retried if an issue appears
        match result {
            // when the upcall was successful
            Some(Ok(Ok(()))) => {}
            // if the upcall coudln't be made (the owning process is registered) (`.schedule upcall`` failed)
            Some(Ok(Err(_err))) => {}
            // if the grant is not available (`.enter` failed)
            Some(Err(_err)) => {}
            // if the owning process is not registered
            None => {}
        }
    }
}
