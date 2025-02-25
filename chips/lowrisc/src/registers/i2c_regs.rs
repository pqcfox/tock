// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright lowRISC contributors (OpenTitan project).

// Generated register constants for i2c.
// Original reference file: hw/ip/i2c/data/i2c.hjson
use kernel::utilities::registers::ReadWrite;
use kernel::utilities::registers::{register_bitfields, register_structs};
/// Depth of FMT, RX, and TX FIFOs.
pub const I2C_PARAM_FIFO_DEPTH: u32 = 64;
/// Depth of ACQ FIFO.
pub const I2C_PARAM_ACQ_FIFO_DEPTH: u32 = 268;
/// Number of alerts
pub const I2C_PARAM_NUM_ALERTS: u32 = 1;
/// Register width
pub const I2C_PARAM_REG_WIDTH: u32 = 32;

register_structs! {
    pub I2cRegisters {
        /// Interrupt State Register
        (0x0000 => pub intr_state: ReadWrite<u32, INTR::Register>),
        /// Interrupt Enable Register
        (0x0004 => pub intr_enable: ReadWrite<u32, INTR::Register>),
        /// Interrupt Test Register
        (0x0008 => pub intr_test: ReadWrite<u32, INTR::Register>),
        /// Alert Test Register
        (0x000c => pub alert_test: ReadWrite<u32, ALERT_TEST::Register>),
        /// I2C Control Register
        (0x0010 => pub ctrl: ReadWrite<u32, CTRL::Register>),
        /// I2C Live Status Register for Host and Target modes
        (0x0014 => pub status: ReadWrite<u32, STATUS::Register>),
        /// I2C Read Data
        (0x0018 => pub rdata: ReadWrite<u32, RDATA::Register>),
        /// I2C Host Format Data
        (0x001c => pub fdata: ReadWrite<u32, FDATA::Register>),
        /// I2C FIFO control register
        (0x0020 => pub fifo_ctrl: ReadWrite<u32, FIFO_CTRL::Register>),
        /// Host mode FIFO configuration
        (0x0024 => pub host_fifo_config: ReadWrite<u32, HOST_FIFO_CONFIG::Register>),
        /// Target mode FIFO configuration
        (0x0028 => pub target_fifo_config: ReadWrite<u32, TARGET_FIFO_CONFIG::Register>),
        /// Host mode FIFO status register
        (0x002c => pub host_fifo_status: ReadWrite<u32, HOST_FIFO_STATUS::Register>),
        /// Target mode FIFO status register
        (0x0030 => pub target_fifo_status: ReadWrite<u32, TARGET_FIFO_STATUS::Register>),
        /// I2C Override Control Register
        (0x0034 => pub ovrd: ReadWrite<u32, OVRD::Register>),
        /// Oversampled RX values
        (0x0038 => pub val: ReadWrite<u32, VAL::Register>),
        /// Detailed I2C Timings (directly corresponding to table 10 in the I2C Specification).
        (0x003c => pub timing0: ReadWrite<u32, TIMING0::Register>),
        /// Detailed I2C Timings (directly corresponding to table 10 in the I2C Specification).
        (0x0040 => pub timing1: ReadWrite<u32, TIMING1::Register>),
        /// Detailed I2C Timings (directly corresponding to table 10 in the I2C Specification).
        (0x0044 => pub timing2: ReadWrite<u32, TIMING2::Register>),
        /// Detailed I2C Timings (directly corresponding to table 10, in the I2C Specification).
        (0x0048 => pub timing3: ReadWrite<u32, TIMING3::Register>),
        /// Detailed I2C Timings (directly corresponding to table 10, in the I2C Specification).
        (0x004c => pub timing4: ReadWrite<u32, TIMING4::Register>),
        /// I2C clock stretching and bus timeout control.
        (0x0050 => pub timeout_ctrl: ReadWrite<u32, TIMEOUT_CTRL::Register>),
        /// I2C target address and mask pairs
        (0x0054 => pub target_id: ReadWrite<u32, TARGET_ID::Register>),
        /// I2C target acquired data
        (0x0058 => pub acqdata: ReadWrite<u32, ACQDATA::Register>),
        /// I2C target transmit data
        (0x005c => pub txdata: ReadWrite<u32, TXDATA::Register>),
        /// I2C host clock generation timeout value (in units of input clock frequency).
        (0x0060 => pub host_timeout_ctrl: ReadWrite<u32, HOST_TIMEOUT_CTRL::Register>),
        /// I2C target internal stretching timeout control.
        (0x0064 => pub target_timeout_ctrl: ReadWrite<u32, TARGET_TIMEOUT_CTRL::Register>),
        /// Number of times the I2C target has NACK'ed a new transaction since the last read of this
        /// register.
        (0x0068 => pub target_nack_count: ReadWrite<u32, TARGET_NACK_COUNT::Register>),
        /// Controls for mid-transfer (N)ACK phase handling
        (0x006c => pub target_ack_ctrl: ReadWrite<u32, TARGET_ACK_CTRL::Register>),
        /// The data byte pending to be written to the ACQ FIFO.
        (0x0070 => pub acq_fifo_next_data: ReadWrite<u32, ACQ_FIFO_NEXT_DATA::Register>),
        /// Timeout in Host-Mode for an unhandled NACK before hardware automatically ends the
        /// transaction.
        (0x0074 => pub host_nack_handler_timeout: ReadWrite<u32, HOST_NACK_HANDLER_TIMEOUT::Register>),
        /// Latched events that explain why the controller halted.
        (0x0078 => pub controller_events: ReadWrite<u32, CONTROLLER_EVENTS::Register>),
        /// Latched events that can cause the target module to stretch the clock at the beginning of a
        /// read transfer.
        (0x007c => pub target_events: ReadWrite<u32, TARGET_EVENTS::Register>),
        (0x0080 => @END),
    }
}

register_bitfields![u32,
    /// Common Interrupt Offsets
    pub INTR [
        FMT_THRESHOLD OFFSET(0) NUMBITS(1) [],
        RX_THRESHOLD OFFSET(1) NUMBITS(1) [],
        ACQ_THRESHOLD OFFSET(2) NUMBITS(1) [],
        RX_OVERFLOW OFFSET(3) NUMBITS(1) [],
        CONTROLLER_HALT OFFSET(4) NUMBITS(1) [],
        SCL_INTERFERENCE OFFSET(5) NUMBITS(1) [],
        SDA_INTERFERENCE OFFSET(6) NUMBITS(1) [],
        STRETCH_TIMEOUT OFFSET(7) NUMBITS(1) [],
        SDA_UNSTABLE OFFSET(8) NUMBITS(1) [],
        CMD_COMPLETE OFFSET(9) NUMBITS(1) [],
        TX_STRETCH OFFSET(10) NUMBITS(1) [],
        TX_THRESHOLD OFFSET(11) NUMBITS(1) [],
        ACQ_STRETCH OFFSET(12) NUMBITS(1) [],
        UNEXP_STOP OFFSET(13) NUMBITS(1) [],
        HOST_TIMEOUT OFFSET(14) NUMBITS(1) [],
    ],
    pub ALERT_TEST [
        FATAL_FAULT OFFSET(0) NUMBITS(1) [],
    ],
    pub CTRL [
        ENABLEHOST OFFSET(0) NUMBITS(1) [],
        ENABLETARGET OFFSET(1) NUMBITS(1) [],
        LLPBK OFFSET(2) NUMBITS(1) [],
        NACK_ADDR_AFTER_TIMEOUT OFFSET(3) NUMBITS(1) [],
        ACK_CTRL_EN OFFSET(4) NUMBITS(1) [],
        MULTI_CONTROLLER_MONITOR_EN OFFSET(5) NUMBITS(1) [],
        TX_STRETCH_CTRL_EN OFFSET(6) NUMBITS(1) [],
    ],
    pub STATUS [
        FMTFULL OFFSET(0) NUMBITS(1) [],
        RXFULL OFFSET(1) NUMBITS(1) [],
        FMTEMPTY OFFSET(2) NUMBITS(1) [],
        HOSTIDLE OFFSET(3) NUMBITS(1) [],
        TARGETIDLE OFFSET(4) NUMBITS(1) [],
        RXEMPTY OFFSET(5) NUMBITS(1) [],
        TXFULL OFFSET(6) NUMBITS(1) [],
        ACQFULL OFFSET(7) NUMBITS(1) [],
        TXEMPTY OFFSET(8) NUMBITS(1) [],
        ACQEMPTY OFFSET(9) NUMBITS(1) [],
        ACK_CTRL_STRETCH OFFSET(10) NUMBITS(1) [],
    ],
    pub RDATA [
        RDATA OFFSET(0) NUMBITS(8) [],
    ],
    pub FDATA [
        FBYTE OFFSET(0) NUMBITS(8) [],
        START OFFSET(8) NUMBITS(1) [],
        STOP OFFSET(9) NUMBITS(1) [],
        READB OFFSET(10) NUMBITS(1) [],
        RCONT OFFSET(11) NUMBITS(1) [],
        NAKOK OFFSET(12) NUMBITS(1) [],
    ],
    pub FIFO_CTRL [
        RXRST OFFSET(0) NUMBITS(1) [],
        FMTRST OFFSET(1) NUMBITS(1) [],
        ACQRST OFFSET(7) NUMBITS(1) [],
        TXRST OFFSET(8) NUMBITS(1) [],
    ],
    pub HOST_FIFO_CONFIG [
        RX_THRESH OFFSET(0) NUMBITS(12) [],
        FMT_THRESH OFFSET(16) NUMBITS(12) [],
    ],
    pub TARGET_FIFO_CONFIG [
        TX_THRESH OFFSET(0) NUMBITS(12) [],
        ACQ_THRESH OFFSET(16) NUMBITS(12) [],
    ],
    pub HOST_FIFO_STATUS [
        FMTLVL OFFSET(0) NUMBITS(12) [],
        RXLVL OFFSET(16) NUMBITS(12) [],
    ],
    pub TARGET_FIFO_STATUS [
        TXLVL OFFSET(0) NUMBITS(12) [],
        ACQLVL OFFSET(16) NUMBITS(12) [],
    ],
    pub OVRD [
        TXOVRDEN OFFSET(0) NUMBITS(1) [],
        SCLVAL OFFSET(1) NUMBITS(1) [],
        SDAVAL OFFSET(2) NUMBITS(1) [],
    ],
    pub VAL [
        SCL_RX OFFSET(0) NUMBITS(16) [],
        SDA_RX OFFSET(16) NUMBITS(16) [],
    ],
    pub TIMING0 [
        THIGH OFFSET(0) NUMBITS(13) [],
        TLOW OFFSET(16) NUMBITS(13) [],
    ],
    pub TIMING1 [
        T_R OFFSET(0) NUMBITS(10) [],
        T_F OFFSET(16) NUMBITS(9) [],
    ],
    pub TIMING2 [
        TSU_STA OFFSET(0) NUMBITS(13) [],
        THD_STA OFFSET(16) NUMBITS(13) [],
    ],
    pub TIMING3 [
        TSU_DAT OFFSET(0) NUMBITS(9) [],
        THD_DAT OFFSET(16) NUMBITS(13) [],
    ],
    pub TIMING4 [
        TSU_STO OFFSET(0) NUMBITS(13) [],
        T_BUF OFFSET(16) NUMBITS(13) [],
    ],
    pub TIMEOUT_CTRL [
        VAL OFFSET(0) NUMBITS(30) [],
        MODE OFFSET(30) NUMBITS(1) [
            STRETCH_TIMEOUT = 0,
            BUS_TIMEOUT = 1,
        ],
        EN OFFSET(31) NUMBITS(1) [],
    ],
    pub TARGET_ID [
        ADDRESS0 OFFSET(0) NUMBITS(7) [],
        MASK0 OFFSET(7) NUMBITS(7) [],
        ADDRESS1 OFFSET(14) NUMBITS(7) [],
        MASK1 OFFSET(21) NUMBITS(7) [],
    ],
    pub ACQDATA [
        ABYTE OFFSET(0) NUMBITS(8) [],
        SIGNAL OFFSET(8) NUMBITS(3) [
            NONE = 0,
            START = 1,
            STOP = 2,
            RESTART = 3,
            NACK = 4,
            NACK_START = 5,
            NACK_STOP = 6,
        ],
    ],
    pub TXDATA [
        TXDATA OFFSET(0) NUMBITS(8) [],
    ],
    pub HOST_TIMEOUT_CTRL [
        HOST_TIMEOUT_CTRL OFFSET(0) NUMBITS(20) [],
    ],
    pub TARGET_TIMEOUT_CTRL [
        VAL OFFSET(0) NUMBITS(31) [],
        EN OFFSET(31) NUMBITS(1) [],
    ],
    pub TARGET_NACK_COUNT [
        TARGET_NACK_COUNT OFFSET(0) NUMBITS(8) [],
    ],
    pub TARGET_ACK_CTRL [
        NBYTES OFFSET(0) NUMBITS(9) [],
        NACK OFFSET(31) NUMBITS(1) [],
    ],
    pub ACQ_FIFO_NEXT_DATA [
        ACQ_FIFO_NEXT_DATA OFFSET(0) NUMBITS(8) [],
    ],
    pub HOST_NACK_HANDLER_TIMEOUT [
        VAL OFFSET(0) NUMBITS(31) [],
        EN OFFSET(31) NUMBITS(1) [],
    ],
    pub CONTROLLER_EVENTS [
        NACK OFFSET(0) NUMBITS(1) [],
        UNHANDLED_NACK_TIMEOUT OFFSET(1) NUMBITS(1) [],
        BUS_TIMEOUT OFFSET(2) NUMBITS(1) [],
        ARBITRATION_LOST OFFSET(3) NUMBITS(1) [],
    ],
    pub TARGET_EVENTS [
        TX_PENDING OFFSET(0) NUMBITS(1) [],
        BUS_TIMEOUT OFFSET(1) NUMBITS(1) [],
        ARBITRATION_LOST OFFSET(2) NUMBITS(1) [],
    ],
];

// End generated register constants for i2c
