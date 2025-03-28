// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright lowRISC contributors (OpenTitan project).

// Generated register constants for adc_ctrl.
// Original reference file: hw/ip/adc_ctrl/data/adc_ctrl.hjson
use kernel::utilities::registers::ReadWrite;
use kernel::utilities::registers::{register_bitfields, register_structs};
/// Number for ADC filters
pub const ADC_CTRL_PARAM_NUM_ADC_FILTER: u32 = 8;
/// Number for ADC channels
pub const ADC_CTRL_PARAM_NUM_ADC_CHANNEL: u32 = 2;
/// Number of alerts
pub const ADC_CTRL_PARAM_NUM_ALERTS: u32 = 1;
/// Register width
pub const ADC_CTRL_PARAM_REG_WIDTH: u32 = 32;

register_structs! {
    pub AdcCtrlRegisters {
        /// Interrupt State Register
        (0x0000 => pub intr_state: ReadWrite<u32, INTR::Register>),
        /// Interrupt Enable Register
        (0x0004 => pub intr_enable: ReadWrite<u32, INTR::Register>),
        /// Interrupt Test Register
        (0x0008 => pub intr_test: ReadWrite<u32, INTR::Register>),
        /// Alert Test Register
        (0x000c => pub alert_test: ReadWrite<u32, ALERT_TEST::Register>),
        /// ADC enable control register
        (0x0010 => pub adc_en_ctl: ReadWrite<u32, ADC_EN_CTL::Register>),
        /// ADC PowerDown(PD) control register
        (0x0014 => pub adc_pd_ctl: ReadWrite<u32, ADC_PD_CTL::Register>),
        /// ADC Low-Power(LP) sample control register
        (0x0018 => pub adc_lp_sample_ctl: ReadWrite<u32, ADC_LP_SAMPLE_CTL::Register>),
        /// ADC sample control register
        (0x001c => pub adc_sample_ctl: ReadWrite<u32, ADC_SAMPLE_CTL::Register>),
        /// ADC FSM reset control
        (0x0020 => pub adc_fsm_rst: ReadWrite<u32, ADC_FSM_RST::Register>),
        /// ADC channel0 filter range
        (0x0024 => pub adc_chn0_filter_ctl: [ReadWrite<u32, ADC_CHN0_FILTER_CTL::Register>; 8]),
        /// ADC channel1 filter range
        (0x0044 => pub adc_chn1_filter_ctl: [ReadWrite<u32, ADC_CHN1_FILTER_CTL::Register>; 8]),
        /// ADC value sampled on channel
        (0x0064 => pub adc_chn_val: [ReadWrite<u32, ADC_CHN_VAL::Register>; 2]),
        /// Enable filter matches as wakeups
        (0x006c => pub adc_wakeup_ctl: ReadWrite<u32, ADC_WAKEUP_CTL::Register>),
        /// Adc filter match status
        (0x0070 => pub filter_status: ReadWrite<u32, FILTER_STATUS::Register>),
        /// Interrupt enable controls.
        (0x0074 => pub adc_intr_ctl: ReadWrite<u32, ADC_INTR_CTL::Register>),
        /// Debug cable internal status
        (0x0078 => pub adc_intr_status: ReadWrite<u32, ADC_INTR_STATUS::Register>),
        /// State of the internal state machine
        (0x007c => pub adc_fsm_state: ReadWrite<u32, ADC_FSM_STATE::Register>),
        (0x0080 => @END),
    }
}

register_bitfields![u32,
    /// Common Interrupt Offsets
    pub INTR [
        MATCH_PENDING OFFSET(0) NUMBITS(1) [],
    ],
    pub ALERT_TEST [
        FATAL_FAULT OFFSET(0) NUMBITS(1) [],
    ],
    pub ADC_EN_CTL [
        ADC_ENABLE OFFSET(0) NUMBITS(1) [],
        ONESHOT_MODE OFFSET(1) NUMBITS(1) [],
    ],
    pub ADC_PD_CTL [
        LP_MODE OFFSET(0) NUMBITS(1) [],
        PWRUP_TIME OFFSET(4) NUMBITS(4) [],
        WAKEUP_TIME OFFSET(8) NUMBITS(24) [],
    ],
    pub ADC_LP_SAMPLE_CTL [
        LP_SAMPLE_CNT OFFSET(0) NUMBITS(8) [],
    ],
    pub ADC_SAMPLE_CTL [
        NP_SAMPLE_CNT OFFSET(0) NUMBITS(16) [],
    ],
    pub ADC_FSM_RST [
        RST_EN OFFSET(0) NUMBITS(1) [],
    ],
    pub ADC_CHN0_FILTER_CTL [
        MIN_V_0 OFFSET(2) NUMBITS(10) [],
        COND_0 OFFSET(12) NUMBITS(1) [],
        MAX_V_0 OFFSET(18) NUMBITS(10) [],
        EN_0 OFFSET(31) NUMBITS(1) [],
    ],
    pub ADC_CHN1_FILTER_CTL [
        MIN_V_0 OFFSET(2) NUMBITS(10) [],
        COND_0 OFFSET(12) NUMBITS(1) [],
        MAX_V_0 OFFSET(18) NUMBITS(10) [],
        EN_0 OFFSET(31) NUMBITS(1) [],
    ],
    pub ADC_CHN_VAL [
        ADC_CHN_VALUE_EXT_0 OFFSET(0) NUMBITS(2) [],
        ADC_CHN_VALUE_0 OFFSET(2) NUMBITS(10) [],
        ADC_CHN_VALUE_INTR_EXT_0 OFFSET(16) NUMBITS(2) [],
        ADC_CHN_VALUE_INTR_0 OFFSET(18) NUMBITS(10) [],
    ],
    pub ADC_WAKEUP_CTL [
        MATCH_EN OFFSET(0) NUMBITS(8) [],
        TRANS_EN OFFSET(8) NUMBITS(1) [],
    ],
    pub FILTER_STATUS [
        MATCH OFFSET(0) NUMBITS(8) [],
        TRANS OFFSET(8) NUMBITS(1) [],
    ],
    pub ADC_INTR_CTL [
        MATCH_EN OFFSET(0) NUMBITS(8) [],
        TRANS_EN OFFSET(8) NUMBITS(1) [],
        ONESHOT_EN OFFSET(9) NUMBITS(1) [],
    ],
    pub ADC_INTR_STATUS [
        MATCH OFFSET(0) NUMBITS(8) [],
        TRANS OFFSET(8) NUMBITS(1) [],
        ONESHOT OFFSET(9) NUMBITS(1) [],
    ],
    pub ADC_FSM_STATE [
        STATE OFFSET(0) NUMBITS(5) [
            PWRDN = 0,
            PWRUP = 1,
            ONEST_0 = 2,
            ONEST_021 = 3,
            ONEST_1 = 4,
            ONEST_DONE = 5,
            LP_0 = 6,
            LP_021 = 7,
            LP_1 = 8,
            LP_EVAL = 9,
            LP_SLP = 10,
            LP_PWRUP = 11,
            NP_0 = 12,
            NP_021 = 13,
            NP_1 = 14,
            NP_EVAL = 15,
            NP_DONE = 16,
        ],
    ],
];

// End generated register constants for adc_ctrl
