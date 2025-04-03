// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright lowRISC contributors (OpenTitan project).

// Generated register constants for usbdev.
// Original reference file: hw/ip/usbdev/data/usbdev.hjson
use kernel::utilities::registers::ReadWrite;
use kernel::utilities::registers::{register_bitfields, register_structs};
/// Number of endpoints
pub const USBDEV_PARAM_N_ENDPOINTS: u32 = 12;
/// Number of alerts
pub const USBDEV_PARAM_NUM_ALERTS: u32 = 1;
/// Register width
pub const USBDEV_PARAM_REG_WIDTH: u32 = 32;

register_structs! {
    pub UsbdevRegisters {
        /// Interrupt State Register
        (0x0000 => pub intr_state: ReadWrite<u32, INTR::Register>),
        /// Interrupt Enable Register
        (0x0004 => pub intr_enable: ReadWrite<u32, INTR::Register>),
        /// Interrupt Test Register
        (0x0008 => pub intr_test: ReadWrite<u32, INTR::Register>),
        /// Alert Test Register
        (0x000c => pub alert_test: ReadWrite<u32, ALERT_TEST::Register>),
        /// USB Control
        (0x0010 => pub usbctrl: ReadWrite<u32, USBCTRL::Register>),
        /// Enable an endpoint to respond to transactions in the downstream direction.
        (0x0014 => pub ep_out_enable: [ReadWrite<u32, EP_OUT_ENABLE::Register>; 1]),
        /// Enable an endpoint to respond to transactions in the upstream direction.
        (0x0018 => pub ep_in_enable: [ReadWrite<u32, EP_IN_ENABLE::Register>; 1]),
        /// USB Status
        (0x001c => pub usbstat: ReadWrite<u32, USBSTAT::Register>),
        /// Available OUT Buffer FIFO
        (0x0020 => pub avoutbuffer: ReadWrite<u32, AVOUTBUFFER::Register>),
        /// Available SETUP Buffer FIFO
        (0x0024 => pub avsetupbuffer: ReadWrite<u32, AVSETUPBUFFER::Register>),
        /// Received Buffer FIFO
        (0x0028 => pub rxfifo: ReadWrite<u32, RXFIFO::Register>),
        /// Receive SETUP transaction enable
        (0x002c => pub rxenable_setup: [ReadWrite<u32, RXENABLE_SETUP::Register>; 1]),
        /// Receive OUT transaction enable
        (0x0030 => pub rxenable_out: [ReadWrite<u32, RXENABLE_OUT::Register>; 1]),
        /// Set NAK after OUT transactions
        (0x0034 => pub set_nak_out: [ReadWrite<u32, SET_NAK_OUT::Register>; 1]),
        /// IN Transaction Sent
        (0x0038 => pub in_sent: [ReadWrite<u32, IN_SENT::Register>; 1]),
        /// OUT Endpoint STALL control
        (0x003c => pub out_stall: [ReadWrite<u32, OUT_STALL::Register>; 1]),
        /// IN Endpoint STALL control
        (0x0040 => pub in_stall: [ReadWrite<u32, IN_STALL::Register>; 1]),
        /// Configure IN Transaction
        (0x0044 => pub configin: [ReadWrite<u32, CONFIGIN::Register>; 12]),
        /// OUT Endpoint isochronous setting
        (0x0074 => pub out_iso: [ReadWrite<u32, OUT_ISO::Register>; 1]),
        /// IN Endpoint isochronous setting
        (0x0078 => pub in_iso: [ReadWrite<u32, IN_ISO::Register>; 1]),
        /// OUT Endpoints Data Toggles
        (0x007c => pub out_data_toggle: ReadWrite<u32, OUT_DATA_TOGGLE::Register>),
        /// IN Endpoints Data Toggles
        (0x0080 => pub in_data_toggle: ReadWrite<u32, IN_DATA_TOGGLE::Register>),
        /// USB PHY pins sense.
        (0x0084 => pub phy_pins_sense: ReadWrite<u32, PHY_PINS_SENSE::Register>),
        /// USB PHY pins drive.
        (0x0088 => pub phy_pins_drive: ReadWrite<u32, PHY_PINS_DRIVE::Register>),
        /// USB PHY Configuration
        (0x008c => pub phy_config: ReadWrite<u32, PHY_CONFIG::Register>),
        /// USB wake module control for suspend / resume
        (0x0090 => pub wake_control: ReadWrite<u32, WAKE_CONTROL::Register>),
        /// USB wake module events and debug
        (0x0094 => pub wake_events: ReadWrite<u32, WAKE_EVENTS::Register>),
        /// FIFO control register
        (0x0098 => pub fifo_ctrl: ReadWrite<u32, FIFO_CTRL::Register>),
        /// Counter for OUT side USB events.
        (0x009c => pub count_out: ReadWrite<u32, COUNT_OUT::Register>),
        /// Counter for IN side USB events.
        (0x00a0 => pub count_in: ReadWrite<u32, COUNT_IN::Register>),
        /// Count of IN transactions for which no packet data was available.
        (0x00a4 => pub count_nodata_in: ReadWrite<u32, COUNT_NODATA_IN::Register>),
        /// Count of error conditions detected on token packets from the host.
        (0x00a8 => pub count_errors: ReadWrite<u32, COUNT_ERRORS::Register>),
        (0x00ac => _reserved1),
        /// Memory area: 2 KiB packet buffer. Divided into thirty two 64-byte buffers.
        (0x0800 => pub buffer: [ReadWrite<u32>; 512]),
        (0x1000 => @END),
    }
}

register_bitfields![u32,
    /// Common Interrupt Offsets
    pub INTR [
        PKT_RECEIVED OFFSET(0) NUMBITS(1) [],
        PKT_SENT OFFSET(1) NUMBITS(1) [],
        DISCONNECTED OFFSET(2) NUMBITS(1) [],
        HOST_LOST OFFSET(3) NUMBITS(1) [],
        LINK_RESET OFFSET(4) NUMBITS(1) [],
        LINK_SUSPEND OFFSET(5) NUMBITS(1) [],
        LINK_RESUME OFFSET(6) NUMBITS(1) [],
        AV_OUT_EMPTY OFFSET(7) NUMBITS(1) [],
        RX_FULL OFFSET(8) NUMBITS(1) [],
        AV_OVERFLOW OFFSET(9) NUMBITS(1) [],
        LINK_IN_ERR OFFSET(10) NUMBITS(1) [],
        RX_CRC_ERR OFFSET(11) NUMBITS(1) [],
        RX_PID_ERR OFFSET(12) NUMBITS(1) [],
        RX_BITSTUFF_ERR OFFSET(13) NUMBITS(1) [],
        FRAME OFFSET(14) NUMBITS(1) [],
        POWERED OFFSET(15) NUMBITS(1) [],
        LINK_OUT_ERR OFFSET(16) NUMBITS(1) [],
        AV_SETUP_EMPTY OFFSET(17) NUMBITS(1) [],
    ],
    pub ALERT_TEST [
        FATAL_FAULT OFFSET(0) NUMBITS(1) [],
    ],
    pub USBCTRL [
        ENABLE OFFSET(0) NUMBITS(1) [],
        RESUME_LINK_ACTIVE OFFSET(1) NUMBITS(1) [],
        DEVICE_ADDRESS OFFSET(16) NUMBITS(7) [],
    ],
    pub EP_OUT_ENABLE [
        ENABLE_0 OFFSET(0) NUMBITS(1) [],
        ENABLE_1 OFFSET(1) NUMBITS(1) [],
        ENABLE_2 OFFSET(2) NUMBITS(1) [],
        ENABLE_3 OFFSET(3) NUMBITS(1) [],
        ENABLE_4 OFFSET(4) NUMBITS(1) [],
        ENABLE_5 OFFSET(5) NUMBITS(1) [],
        ENABLE_6 OFFSET(6) NUMBITS(1) [],
        ENABLE_7 OFFSET(7) NUMBITS(1) [],
        ENABLE_8 OFFSET(8) NUMBITS(1) [],
        ENABLE_9 OFFSET(9) NUMBITS(1) [],
        ENABLE_10 OFFSET(10) NUMBITS(1) [],
        ENABLE_11 OFFSET(11) NUMBITS(1) [],
    ],
    pub EP_IN_ENABLE [
        ENABLE_0 OFFSET(0) NUMBITS(1) [],
        ENABLE_1 OFFSET(1) NUMBITS(1) [],
        ENABLE_2 OFFSET(2) NUMBITS(1) [],
        ENABLE_3 OFFSET(3) NUMBITS(1) [],
        ENABLE_4 OFFSET(4) NUMBITS(1) [],
        ENABLE_5 OFFSET(5) NUMBITS(1) [],
        ENABLE_6 OFFSET(6) NUMBITS(1) [],
        ENABLE_7 OFFSET(7) NUMBITS(1) [],
        ENABLE_8 OFFSET(8) NUMBITS(1) [],
        ENABLE_9 OFFSET(9) NUMBITS(1) [],
        ENABLE_10 OFFSET(10) NUMBITS(1) [],
        ENABLE_11 OFFSET(11) NUMBITS(1) [],
    ],
    pub USBSTAT [
        FRAME OFFSET(0) NUMBITS(11) [],
        HOST_LOST OFFSET(11) NUMBITS(1) [],
        LINK_STATE OFFSET(12) NUMBITS(3) [
            DISCONNECTED = 0,
            POWERED = 1,
            POWERED_SUSPENDED = 2,
            ACTIVE = 3,
            SUSPENDED = 4,
            ACTIVE_NOSOF = 5,
            RESUMING = 6,
        ],
        SENSE OFFSET(15) NUMBITS(1) [],
        AV_OUT_DEPTH OFFSET(16) NUMBITS(4) [],
        AV_SETUP_DEPTH OFFSET(20) NUMBITS(3) [],
        AV_OUT_FULL OFFSET(23) NUMBITS(1) [],
        RX_DEPTH OFFSET(24) NUMBITS(4) [],
        AV_SETUP_FULL OFFSET(30) NUMBITS(1) [],
        RX_EMPTY OFFSET(31) NUMBITS(1) [],
    ],
    pub AVOUTBUFFER [
        BUFFER OFFSET(0) NUMBITS(5) [],
    ],
    pub AVSETUPBUFFER [
        BUFFER OFFSET(0) NUMBITS(5) [],
    ],
    pub RXFIFO [
        BUFFER OFFSET(0) NUMBITS(5) [],
        SIZE OFFSET(8) NUMBITS(7) [],
        SETUP OFFSET(19) NUMBITS(1) [],
        EP OFFSET(20) NUMBITS(4) [],
    ],
    pub RXENABLE_SETUP [
        SETUP_0 OFFSET(0) NUMBITS(1) [],
        SETUP_1 OFFSET(1) NUMBITS(1) [],
        SETUP_2 OFFSET(2) NUMBITS(1) [],
        SETUP_3 OFFSET(3) NUMBITS(1) [],
        SETUP_4 OFFSET(4) NUMBITS(1) [],
        SETUP_5 OFFSET(5) NUMBITS(1) [],
        SETUP_6 OFFSET(6) NUMBITS(1) [],
        SETUP_7 OFFSET(7) NUMBITS(1) [],
        SETUP_8 OFFSET(8) NUMBITS(1) [],
        SETUP_9 OFFSET(9) NUMBITS(1) [],
        SETUP_10 OFFSET(10) NUMBITS(1) [],
        SETUP_11 OFFSET(11) NUMBITS(1) [],
    ],
    pub RXENABLE_OUT [
        OUT_0 OFFSET(0) NUMBITS(1) [],
        OUT_1 OFFSET(1) NUMBITS(1) [],
        OUT_2 OFFSET(2) NUMBITS(1) [],
        OUT_3 OFFSET(3) NUMBITS(1) [],
        OUT_4 OFFSET(4) NUMBITS(1) [],
        OUT_5 OFFSET(5) NUMBITS(1) [],
        OUT_6 OFFSET(6) NUMBITS(1) [],
        OUT_7 OFFSET(7) NUMBITS(1) [],
        OUT_8 OFFSET(8) NUMBITS(1) [],
        OUT_9 OFFSET(9) NUMBITS(1) [],
        OUT_10 OFFSET(10) NUMBITS(1) [],
        OUT_11 OFFSET(11) NUMBITS(1) [],
    ],
    pub SET_NAK_OUT [
        ENABLE_0 OFFSET(0) NUMBITS(1) [],
        ENABLE_1 OFFSET(1) NUMBITS(1) [],
        ENABLE_2 OFFSET(2) NUMBITS(1) [],
        ENABLE_3 OFFSET(3) NUMBITS(1) [],
        ENABLE_4 OFFSET(4) NUMBITS(1) [],
        ENABLE_5 OFFSET(5) NUMBITS(1) [],
        ENABLE_6 OFFSET(6) NUMBITS(1) [],
        ENABLE_7 OFFSET(7) NUMBITS(1) [],
        ENABLE_8 OFFSET(8) NUMBITS(1) [],
        ENABLE_9 OFFSET(9) NUMBITS(1) [],
        ENABLE_10 OFFSET(10) NUMBITS(1) [],
        ENABLE_11 OFFSET(11) NUMBITS(1) [],
    ],
    pub IN_SENT [
        SENT_0 OFFSET(0) NUMBITS(1) [],
        SENT_1 OFFSET(1) NUMBITS(1) [],
        SENT_2 OFFSET(2) NUMBITS(1) [],
        SENT_3 OFFSET(3) NUMBITS(1) [],
        SENT_4 OFFSET(4) NUMBITS(1) [],
        SENT_5 OFFSET(5) NUMBITS(1) [],
        SENT_6 OFFSET(6) NUMBITS(1) [],
        SENT_7 OFFSET(7) NUMBITS(1) [],
        SENT_8 OFFSET(8) NUMBITS(1) [],
        SENT_9 OFFSET(9) NUMBITS(1) [],
        SENT_10 OFFSET(10) NUMBITS(1) [],
        SENT_11 OFFSET(11) NUMBITS(1) [],
    ],
    pub OUT_STALL [
        ENDPOINT_0 OFFSET(0) NUMBITS(1) [],
        ENDPOINT_1 OFFSET(1) NUMBITS(1) [],
        ENDPOINT_2 OFFSET(2) NUMBITS(1) [],
        ENDPOINT_3 OFFSET(3) NUMBITS(1) [],
        ENDPOINT_4 OFFSET(4) NUMBITS(1) [],
        ENDPOINT_5 OFFSET(5) NUMBITS(1) [],
        ENDPOINT_6 OFFSET(6) NUMBITS(1) [],
        ENDPOINT_7 OFFSET(7) NUMBITS(1) [],
        ENDPOINT_8 OFFSET(8) NUMBITS(1) [],
        ENDPOINT_9 OFFSET(9) NUMBITS(1) [],
        ENDPOINT_10 OFFSET(10) NUMBITS(1) [],
        ENDPOINT_11 OFFSET(11) NUMBITS(1) [],
    ],
    pub IN_STALL [
        ENDPOINT_0 OFFSET(0) NUMBITS(1) [],
        ENDPOINT_1 OFFSET(1) NUMBITS(1) [],
        ENDPOINT_2 OFFSET(2) NUMBITS(1) [],
        ENDPOINT_3 OFFSET(3) NUMBITS(1) [],
        ENDPOINT_4 OFFSET(4) NUMBITS(1) [],
        ENDPOINT_5 OFFSET(5) NUMBITS(1) [],
        ENDPOINT_6 OFFSET(6) NUMBITS(1) [],
        ENDPOINT_7 OFFSET(7) NUMBITS(1) [],
        ENDPOINT_8 OFFSET(8) NUMBITS(1) [],
        ENDPOINT_9 OFFSET(9) NUMBITS(1) [],
        ENDPOINT_10 OFFSET(10) NUMBITS(1) [],
        ENDPOINT_11 OFFSET(11) NUMBITS(1) [],
    ],
    pub CONFIGIN [
        BUFFER_0 OFFSET(0) NUMBITS(5) [],
        SIZE_0 OFFSET(8) NUMBITS(7) [],
        SENDING_0 OFFSET(29) NUMBITS(1) [],
        PEND_0 OFFSET(30) NUMBITS(1) [],
        RDY_0 OFFSET(31) NUMBITS(1) [],
    ],
    pub OUT_ISO [
        ISO_0 OFFSET(0) NUMBITS(1) [],
        ISO_1 OFFSET(1) NUMBITS(1) [],
        ISO_2 OFFSET(2) NUMBITS(1) [],
        ISO_3 OFFSET(3) NUMBITS(1) [],
        ISO_4 OFFSET(4) NUMBITS(1) [],
        ISO_5 OFFSET(5) NUMBITS(1) [],
        ISO_6 OFFSET(6) NUMBITS(1) [],
        ISO_7 OFFSET(7) NUMBITS(1) [],
        ISO_8 OFFSET(8) NUMBITS(1) [],
        ISO_9 OFFSET(9) NUMBITS(1) [],
        ISO_10 OFFSET(10) NUMBITS(1) [],
        ISO_11 OFFSET(11) NUMBITS(1) [],
    ],
    pub IN_ISO [
        ISO_0 OFFSET(0) NUMBITS(1) [],
        ISO_1 OFFSET(1) NUMBITS(1) [],
        ISO_2 OFFSET(2) NUMBITS(1) [],
        ISO_3 OFFSET(3) NUMBITS(1) [],
        ISO_4 OFFSET(4) NUMBITS(1) [],
        ISO_5 OFFSET(5) NUMBITS(1) [],
        ISO_6 OFFSET(6) NUMBITS(1) [],
        ISO_7 OFFSET(7) NUMBITS(1) [],
        ISO_8 OFFSET(8) NUMBITS(1) [],
        ISO_9 OFFSET(9) NUMBITS(1) [],
        ISO_10 OFFSET(10) NUMBITS(1) [],
        ISO_11 OFFSET(11) NUMBITS(1) [],
    ],
    pub OUT_DATA_TOGGLE [
        STATUS OFFSET(0) NUMBITS(12) [],
        MASK OFFSET(16) NUMBITS(12) [],
    ],
    pub IN_DATA_TOGGLE [
        STATUS OFFSET(0) NUMBITS(12) [],
        MASK OFFSET(16) NUMBITS(12) [],
    ],
    pub PHY_PINS_SENSE [
        RX_DP_I OFFSET(0) NUMBITS(1) [],
        RX_DN_I OFFSET(1) NUMBITS(1) [],
        RX_D_I OFFSET(2) NUMBITS(1) [],
        TX_DP_O OFFSET(8) NUMBITS(1) [],
        TX_DN_O OFFSET(9) NUMBITS(1) [],
        TX_D_O OFFSET(10) NUMBITS(1) [],
        TX_SE0_O OFFSET(11) NUMBITS(1) [],
        TX_OE_O OFFSET(12) NUMBITS(1) [],
        PWR_SENSE OFFSET(16) NUMBITS(1) [],
    ],
    pub PHY_PINS_DRIVE [
        DP_O OFFSET(0) NUMBITS(1) [],
        DN_O OFFSET(1) NUMBITS(1) [],
        D_O OFFSET(2) NUMBITS(1) [],
        SE0_O OFFSET(3) NUMBITS(1) [],
        OE_O OFFSET(4) NUMBITS(1) [],
        RX_ENABLE_O OFFSET(5) NUMBITS(1) [],
        DP_PULLUP_EN_O OFFSET(6) NUMBITS(1) [],
        DN_PULLUP_EN_O OFFSET(7) NUMBITS(1) [],
        EN OFFSET(16) NUMBITS(1) [],
    ],
    pub PHY_CONFIG [
        USE_DIFF_RCVR OFFSET(0) NUMBITS(1) [],
        TX_USE_D_SE0 OFFSET(1) NUMBITS(1) [],
        EOP_SINGLE_BIT OFFSET(2) NUMBITS(1) [],
        PINFLIP OFFSET(5) NUMBITS(1) [],
        USB_REF_DISABLE OFFSET(6) NUMBITS(1) [],
        TX_OSC_TEST_MODE OFFSET(7) NUMBITS(1) [],
    ],
    pub WAKE_CONTROL [
        SUSPEND_REQ OFFSET(0) NUMBITS(1) [],
        WAKE_ACK OFFSET(1) NUMBITS(1) [],
    ],
    pub WAKE_EVENTS [
        MODULE_ACTIVE OFFSET(0) NUMBITS(1) [],
        DISCONNECTED OFFSET(8) NUMBITS(1) [],
        BUS_RESET OFFSET(9) NUMBITS(1) [],
        BUS_NOT_IDLE OFFSET(10) NUMBITS(1) [],
    ],
    pub FIFO_CTRL [
        AVOUT_RST OFFSET(0) NUMBITS(1) [],
        AVSETUP_RST OFFSET(1) NUMBITS(1) [],
        RX_RST OFFSET(2) NUMBITS(1) [],
    ],
    pub COUNT_OUT [
        COUNT OFFSET(0) NUMBITS(8) [],
        DATATOG_OUT OFFSET(12) NUMBITS(1) [],
        DROP_RX OFFSET(13) NUMBITS(1) [],
        DROP_AVOUT OFFSET(14) NUMBITS(1) [],
        IGN_AVSETUP OFFSET(15) NUMBITS(1) [],
        ENDPOINTS OFFSET(16) NUMBITS(12) [],
        RST OFFSET(31) NUMBITS(1) [],
    ],
    pub COUNT_IN [
        COUNT OFFSET(0) NUMBITS(8) [],
        NODATA OFFSET(13) NUMBITS(1) [],
        NAK OFFSET(14) NUMBITS(1) [],
        TIMEOUT OFFSET(15) NUMBITS(1) [],
        ENDPOINTS OFFSET(16) NUMBITS(12) [],
        RST OFFSET(31) NUMBITS(1) [],
    ],
    pub COUNT_NODATA_IN [
        COUNT OFFSET(0) NUMBITS(8) [],
        ENDPOINTS OFFSET(16) NUMBITS(12) [],
        RST OFFSET(31) NUMBITS(1) [],
    ],
    pub COUNT_ERRORS [
        COUNT OFFSET(0) NUMBITS(8) [],
        PID_INVALID OFFSET(27) NUMBITS(1) [],
        BITSTUFF OFFSET(28) NUMBITS(1) [],
        CRC16 OFFSET(29) NUMBITS(1) [],
        CRC5 OFFSET(30) NUMBITS(1) [],
        RST OFFSET(31) NUMBITS(1) [],
    ],
];

// End generated register constants for usbdev
