// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright lowRISC contributors (OpenTitan project).

// Generated register constants for uart.
// Original reference file: hw/ip/uart/data/uart.hjson
use kernel::utilities::registers::ReadWrite;
use kernel::utilities::registers::{register_bitfields, register_structs};
/// Number of bytes in the RX FIFO.
pub const UART_PARAM_RX_FIFO_DEPTH: u32 = 64;
/// Number of bytes in the TX FIFO.
pub const UART_PARAM_TX_FIFO_DEPTH: u32 = 32;
/// Number of alerts
pub const UART_PARAM_NUM_ALERTS: u32 = 1;
/// Register width
pub const UART_PARAM_REG_WIDTH: u32 = 32;

register_structs! {
    pub UartRegisters {
        /// Interrupt State Register
        (0x0000 => pub intr_state: ReadWrite<u32, INTR::Register>),
        /// Interrupt Enable Register
        (0x0004 => pub intr_enable: ReadWrite<u32, INTR::Register>),
        /// Interrupt Test Register
        (0x0008 => pub intr_test: ReadWrite<u32, INTR::Register>),
        /// Alert Test Register
        (0x000c => pub alert_test: ReadWrite<u32, ALERT_TEST::Register>),
        /// UART control register
        (0x0010 => pub ctrl: ReadWrite<u32, CTRL::Register>),
        /// UART live status register
        (0x0014 => pub status: ReadWrite<u32, STATUS::Register>),
        /// UART read data
        (0x0018 => pub rdata: ReadWrite<u32, RDATA::Register>),
        /// UART write data
        (0x001c => pub wdata: ReadWrite<u32, WDATA::Register>),
        /// UART FIFO control register
        (0x0020 => pub fifo_ctrl: ReadWrite<u32, FIFO_CTRL::Register>),
        /// UART FIFO status register
        (0x0024 => pub fifo_status: ReadWrite<u32, FIFO_STATUS::Register>),
        /// TX pin override control. Gives direct SW control over TX pin state
        (0x0028 => pub ovrd: ReadWrite<u32, OVRD::Register>),
        /// UART oversampled values
        (0x002c => pub val: ReadWrite<u32, VAL::Register>),
        /// UART RX timeout control
        (0x0030 => pub timeout_ctrl: ReadWrite<u32, TIMEOUT_CTRL::Register>),
        (0x0034 => @END),
    }
}

register_bitfields![u32,
    /// Common Interrupt Offsets
    pub INTR [
        TX_WATERMARK OFFSET(0) NUMBITS(1) [],
        RX_WATERMARK OFFSET(1) NUMBITS(1) [],
        TX_DONE OFFSET(2) NUMBITS(1) [],
        RX_OVERFLOW OFFSET(3) NUMBITS(1) [],
        RX_FRAME_ERR OFFSET(4) NUMBITS(1) [],
        RX_BREAK_ERR OFFSET(5) NUMBITS(1) [],
        RX_TIMEOUT OFFSET(6) NUMBITS(1) [],
        RX_PARITY_ERR OFFSET(7) NUMBITS(1) [],
        TX_EMPTY OFFSET(8) NUMBITS(1) [],
    ],
    pub ALERT_TEST [
        FATAL_FAULT OFFSET(0) NUMBITS(1) [],
    ],
    pub CTRL [
        TX OFFSET(0) NUMBITS(1) [],
        RX OFFSET(1) NUMBITS(1) [],
        NF OFFSET(2) NUMBITS(1) [],
        SLPBK OFFSET(4) NUMBITS(1) [],
        LLPBK OFFSET(5) NUMBITS(1) [],
        PARITY_EN OFFSET(6) NUMBITS(1) [],
        PARITY_ODD OFFSET(7) NUMBITS(1) [],
        RXBLVL OFFSET(8) NUMBITS(2) [
            BREAK2 = 0,
            BREAK4 = 1,
            BREAK8 = 2,
            BREAK16 = 3,
        ],
        NCO OFFSET(16) NUMBITS(16) [],
    ],
    pub STATUS [
        TXFULL OFFSET(0) NUMBITS(1) [],
        RXFULL OFFSET(1) NUMBITS(1) [],
        TXEMPTY OFFSET(2) NUMBITS(1) [],
        TXIDLE OFFSET(3) NUMBITS(1) [],
        RXIDLE OFFSET(4) NUMBITS(1) [],
        RXEMPTY OFFSET(5) NUMBITS(1) [],
    ],
    pub RDATA [
        RDATA OFFSET(0) NUMBITS(8) [],
    ],
    pub WDATA [
        WDATA OFFSET(0) NUMBITS(8) [],
    ],
    pub FIFO_CTRL [
        RXRST OFFSET(0) NUMBITS(1) [],
        TXRST OFFSET(1) NUMBITS(1) [],
        RXILVL OFFSET(2) NUMBITS(3) [
            RXLVL1 = 0,
            RXLVL2 = 1,
            RXLVL4 = 2,
            RXLVL8 = 3,
            RXLVL16 = 4,
            RXLVL32 = 5,
            RXLVL62 = 6,
        ],
        TXILVL OFFSET(5) NUMBITS(3) [
            TXLVL1 = 0,
            TXLVL2 = 1,
            TXLVL4 = 2,
            TXLVL8 = 3,
            TXLVL16 = 4,
        ],
    ],
    pub FIFO_STATUS [
        TXLVL OFFSET(0) NUMBITS(8) [],
        RXLVL OFFSET(16) NUMBITS(8) [],
    ],
    pub OVRD [
        TXEN OFFSET(0) NUMBITS(1) [],
        TXVAL OFFSET(1) NUMBITS(1) [],
    ],
    pub VAL [
        RX OFFSET(0) NUMBITS(16) [],
    ],
    pub TIMEOUT_CTRL [
        VAL OFFSET(0) NUMBITS(24) [],
        EN OFFSET(31) NUMBITS(1) [],
    ],
];

// End generated register constants for uart
