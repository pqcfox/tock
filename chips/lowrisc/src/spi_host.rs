// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Serial Peripheral Interface (SPI) Host Driver
use crate::registers::spi_host_regs::{
    SpiHostRegisters, COMMAND, CONFIGOPTS, CONTROL, CSID, ERROR_ENABLE, ERROR_STATUS, EVENT_ENABLE,
    INTR, STATUS,
};
use core::cell::Cell;
use core::cmp;
use kernel::hil;
use kernel::hil::spi::SpiMaster;
use kernel::hil::spi::{ClockPhase, ClockPolarity};
use kernel::utilities::cells::{MapCell, OptionalCell};
use kernel::utilities::leasable_buffer::SubSliceMut;
use kernel::utilities::registers::interfaces::{ReadWriteable, Readable, Writeable};
use kernel::utilities::StaticRef;
use kernel::ErrorCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpiHostStatus {
    SpiTransferCmplt,
    SpiTransferInprog,
}

pub struct SpiHost<'a> {
    registers: StaticRef<SpiHostRegisters>,
    client: OptionalCell<&'a dyn hil::spi::SpiMasterClient>,
    busy: Cell<bool>,
    cpu_clk: u32,
    tsclk: Cell<u32>,
    tx_buf: MapCell<SubSliceMut<'static, u8>>,
    rx_buf: MapCell<SubSliceMut<'static, u8>>,
    tx_len: Cell<usize>,
    rx_len: Cell<usize>,
    tx_offset: Cell<usize>,
    rx_offset: Cell<usize>,
}
// SPI Host Command Direction: Bidirectional
const SPI_HOST_CMD_BIDIRECTIONAL: u32 = 3;
// SPI Host Command Speed: Standard SPI
const SPI_HOST_CMD_STANDARD_SPI: u32 = 0;

impl SpiHost<'_> {
    pub fn new(base: StaticRef<SpiHostRegisters>, cpu_clk: u32) -> Self {
        SpiHost {
            registers: base,
            client: OptionalCell::empty(),
            busy: Cell::new(false),
            cpu_clk,
            tsclk: Cell::new(0),
            tx_buf: MapCell::empty(),
            rx_buf: MapCell::empty(),
            tx_len: Cell::new(0),
            rx_len: Cell::new(0),
            tx_offset: Cell::new(0),
            rx_offset: Cell::new(0),
        }
    }

    pub fn handle_interrupt(&self) {
        let regs = self.registers;
        let irq = regs.intr_state.extract();
        self.disable_interrupts();

        if irq.is_set(INTR::ERROR) {
            //Clear all pending errors.
            self.clear_err_interrupt();
            //Something went wrong, reset IP and clear buffers
            self.reset_spi_ip();
            self.reset_internal_state();
            //r/w_done() may call r/w_bytes() to re-attempt transfer
            self.client.map(|client| match self.tx_buf.take() {
                None => (),
                Some(tx_buf) => {
                    client.read_write_done(tx_buf, self.rx_buf.take(), Err(ErrorCode::FAIL))
                }
            });
            return;
        }

        if irq.is_set(INTR::SPI_EVENT) {
            let status = regs.status.extract();
            self.clear_event_interrupt();

            //This could be set at init, so only follow through
            //once a transfer has started (is_busy())
            if status.is_set(STATUS::TXEMPTY) && self.is_busy() {
                match self.continue_transfer() {
                    Ok(SpiHostStatus::SpiTransferCmplt) => {
                        // Transfer success
                        self.client.map(|client| match self.tx_buf.take() {
                            None => (),
                            Some(tx_buf) => client.read_write_done(
                                tx_buf,
                                self.rx_buf.take(),
                                Ok(self.tx_len.get()),
                            ),
                        });

                        self.disable_tx_interrupt();
                        self.reset_internal_state();
                    }
                    Ok(SpiHostStatus::SpiTransferInprog) => {}
                    Err(err) => {
                        //Transfer failed, lets clean up
                        //Clear all pending interrupts.
                        self.clear_err_interrupt();
                        //Something went wrong, reset IP and clear buffers
                        self.reset_spi_ip();
                        self.reset_internal_state();
                        self.client.map(|client| match self.tx_buf.take() {
                            None => (),
                            Some(tx_buf) => {
                                client.read_write_done(tx_buf, self.rx_buf.take(), Err(err))
                            }
                        });
                    }
                }
            } else {
                self.enable_interrupts();
            }
        }
    }

    //Determine if transfer complete or if we need to keep
    //writing from an offset.
    fn continue_transfer(&self) -> Result<SpiHostStatus, ErrorCode> {
        let rc = self
            .rx_buf
            .take()
            .map(|mut rx_buf| -> Result<SpiHostStatus, ErrorCode> {
                let regs = self.registers;
                let mut val32: u32;
                let mut val8: u8;
                let mut shift_mask;
                let rx_len = self.tx_offset.get() - self.rx_offset.get();
                let read_cycles = self.div_up(rx_len, 4);

                //Receive rx_data (Only 4byte reads are supported)
                for _n in 0..read_cycles {
                    val32 = regs.rxdata[0].get();
                    shift_mask = 0xFF;
                    for i in 0..4 {
                        if self.rx_offset.get() >= self.rx_len.get() {
                            break;
                        }
                        val8 = ((val32 & shift_mask) >> (i * 8)) as u8;
                        if let Some(ptr) = rx_buf.as_slice().get_mut(self.rx_offset.get()) {
                            *ptr = val8;
                        } else {
                            // We have run out of rx buffer size
                            break;
                        }
                        self.rx_offset.set(self.rx_offset.get() + 1);
                        shift_mask <<= 8;
                    }
                }
                //Save buffer!
                self.rx_buf.replace(rx_buf);
                //Transfer was complete */
                if self.tx_offset.get() == self.tx_len.get() {
                    Ok(SpiHostStatus::SpiTransferCmplt)
                } else {
                    //Theres more to transfer, continue writing from the offset
                    self.spi_transfer_progress()
                }
            })
            .map_or_else(|| Err(ErrorCode::FAIL), |rc| rc);

        rc
    }

    /// Continue SPI transfer from offset point
    fn spi_transfer_progress(&self) -> Result<SpiHostStatus, ErrorCode> {
        let mut transfer_complete = false;
        if self
            .tx_buf
            .take()
            .map(|mut tx_buf| -> Result<(), ErrorCode> {
                let regs = self.registers;
                let mut t_byte: u32;
                let mut tx_slice: [u8; 4];

                if regs.status.read(STATUS::TXQD) != 0 || regs.status.read(STATUS::ACTIVE) != 0 {
                    self.tx_buf.replace(tx_buf);
                    return Err(ErrorCode::BUSY);
                }

                while !regs.status.is_set(STATUS::TXFULL) && regs.status.read(STATUS::TXQD) < 64 {
                    tx_slice = [0, 0, 0, 0];
                    for elem in tx_slice.iter_mut() {
                        if self.tx_offset.get() >= self.tx_len.get() {
                            break;
                        }
                        if let Some(val) = tx_buf.as_slice().get(self.tx_offset.get()) {
                            *elem = *val;
                            self.tx_offset.set(self.tx_offset.get() + 1);
                        } else {
                            //Unexpectedly ran out of tx buffer
                            break;
                        }
                    }
                    t_byte = u32::from_le_bytes(tx_slice);
                    regs.txdata[0].set(t_byte);

                    //Transfer Complete in one-shot
                    if self.tx_offset.get() >= self.tx_len.get() {
                        transfer_complete = true;
                        break;
                    }
                }

                //Hold tx_buf for offset transfer continue
                self.tx_buf.replace(tx_buf);

                //Set command register to init transfer
                self.start_transceive();
                Ok(())
            })
            .transpose()
            .is_err()
        {
            return Err(ErrorCode::BUSY);
        }

        if transfer_complete {
            Ok(SpiHostStatus::SpiTransferCmplt)
        } else {
            Ok(SpiHostStatus::SpiTransferInprog)
        }
    }

    /// Issue a command to start SPI transaction
    /// Currently only Bi-Directional transactions are supported
    fn start_transceive(&self) {
        let regs = self.registers;
        //TXQD holds number of 32bit words
        let txfifo_num_bytes = regs.status.read(STATUS::TXQD) * 4;

        //8-bits that describe command transfer len (cannot exceed 255)
        let num_transfer_bytes: u32 = if txfifo_num_bytes > u8::MAX as u32 {
            u8::MAX as u32
        } else {
            txfifo_num_bytes
        };

        self.enable_interrupts();
        self.enable_tx_interrupt();

        //Flush all data in TXFIFO and assert CSAAT for all
        // but the last transfer segment.
        if self.tx_offset.get() >= self.tx_len.get() {
            regs.command.write(
                COMMAND::LEN.val(num_transfer_bytes)
                    + COMMAND::DIRECTION.val(SPI_HOST_CMD_BIDIRECTIONAL)
                    + COMMAND::CSAAT::CLEAR
                    + COMMAND::SPEED.val(SPI_HOST_CMD_STANDARD_SPI),
            );
        } else {
            regs.command.write(
                COMMAND::LEN.val(num_transfer_bytes)
                    + COMMAND::DIRECTION.val(SPI_HOST_CMD_BIDIRECTIONAL)
                    + COMMAND::CSAAT::SET
                    + COMMAND::SPEED.val(SPI_HOST_CMD_STANDARD_SPI),
            );
        }
    }

    /// Reset the soft internal state, should be called once
    /// a spi transaction has been completed.
    fn reset_internal_state(&self) {
        self.clear_spi_busy();
        self.tx_len.set(0);
        self.rx_len.set(0);
        self.tx_offset.set(0);
        self.rx_offset.set(0);

        debug_assert!(self.tx_buf.is_none());
        debug_assert!(self.rx_buf.is_none());
    }

    /// Enable SPI_HOST IP
    /// `dead_code` to silence warnings when not building for mainline qemu
    #[allow(dead_code)]
    fn enable_spi_host(&self) {
        let regs = self.registers;
        //Enables the SPI host
        regs.control
            .modify(CONTROL::SPIEN::SET + CONTROL::OUTPUT_EN::SET);
    }

    /// Reset SPI Host
    fn reset_spi_ip(&self) {
        let regs = self.registers;
        //IP to reset state
        regs.control.modify(CONTROL::SW_RST::SET);

        //Wait for status ready to be set before continuing
        while regs.status.is_set(STATUS::ACTIVE) {}
        //Wait for both FIFOs to completely drain
        while regs.status.read(STATUS::TXQD) != 0 && regs.status.read(STATUS::RXQD) != 0 {}
        //Clear Reset
        regs.control.modify(CONTROL::SW_RST::CLEAR);
    }

    /// Enable both event/err IRQ
    fn enable_interrupts(&self) {
        self.registers
            .intr_state
            .write(INTR::ERROR::SET + INTR::SPI_EVENT::SET);
        self.registers
            .intr_enable
            .modify(INTR::ERROR::SET + INTR::SPI_EVENT::SET);
    }

    /// Disable both event/err IRQ
    fn disable_interrupts(&self) {
        let regs = self.registers;
        regs.intr_enable
            .write(INTR::ERROR::CLEAR + INTR::SPI_EVENT::CLEAR);
    }

    /// Clear the error IRQ
    fn clear_err_interrupt(&self) {
        let regs = self.registers;
        //Clear Error Masks (rw1c)
        regs.error_status.modify(ERROR_STATUS::CMDBUSY::SET);
        regs.error_status.modify(ERROR_STATUS::OVERFLOW::SET);
        regs.error_status.modify(ERROR_STATUS::UNDERFLOW::SET);
        regs.error_status.modify(ERROR_STATUS::CMDINVAL::SET);
        regs.error_status.modify(ERROR_STATUS::CSIDINVAL::SET);
        regs.error_status.modify(ERROR_STATUS::ACCESSINVAL::SET);
        //Clear Error IRQ
        regs.intr_state.modify(INTR::ERROR::SET);
    }

    /// Clear the event IRQ
    fn clear_event_interrupt(&self) {
        let regs = self.registers;
        regs.intr_state.modify(INTR::SPI_EVENT::SET);
    }
    /// Will generate a `test` interrupt on the error irq
    /// Note: Left to allow debug accessibility
    #[allow(dead_code)]
    fn test_error_interrupt(&self) {
        let regs = self.registers;
        regs.intr_test.write(INTR::ERROR::SET);
    }
    /// Clear test interrupts
    /// Note: Left to allow debug accessibility
    #[allow(dead_code)]
    fn clear_tests(&self) {
        let regs = self.registers;
        regs.intr_test
            .write(INTR::ERROR::CLEAR + INTR::SPI_EVENT::CLEAR);
    }

    /// Will generate a `test` interrupt on the event irq
    /// Note: Left to allow debug accessibility
    #[allow(dead_code)]
    fn test_event_interrupt(&self) {
        let regs = self.registers;
        regs.intr_test.write(INTR::SPI_EVENT::SET);
    }

    /// Enable required `event interrupts`
    /// `dead_code` to silence warnings when not building for mainline qemu
    #[allow(dead_code)]
    fn event_enable(&self) {
        let regs = self.registers;
        regs.event_enable.write(EVENT_ENABLE::TXEMPTY::SET);
    }

    fn disable_tx_interrupt(&self) {
        let regs = self.registers;
        regs.event_enable.modify(EVENT_ENABLE::TXEMPTY::CLEAR);
    }

    fn enable_tx_interrupt(&self) {
        let regs = self.registers;
        regs.event_enable.modify(EVENT_ENABLE::TXEMPTY::SET);
    }

    /// Enable required error interrupts
    /// `dead_code` to silence warnings when not building for mainline qemu
    #[allow(dead_code)]
    fn err_enable(&self) {
        let regs = self.registers;
        regs.error_enable.modify(
            ERROR_ENABLE::CMDBUSY::SET
                + ERROR_ENABLE::CMDINVAL::SET
                + ERROR_ENABLE::CSIDINVAL::SET
                + ERROR_ENABLE::OVERFLOW::SET
                + ERROR_ENABLE::UNDERFLOW::SET,
        );
    }

    fn set_spi_busy(&self) {
        self.busy.set(true);
    }

    fn clear_spi_busy(&self) {
        self.busy.set(false);
    }

    /// Divide a/b and return a value always rounded
    /// up to the nearest integer
    fn div_up(&self, a: usize, b: usize) -> usize {
        a.div_ceil(b)
    }

    /// Calculate the scaler based on a specified tsclk rate
    /// This scaler will pre-scale the cpu_clk and must be <= cpu_clk/2
    fn calculate_tsck_scaler(&self, rate: u32) -> Result<u16, ErrorCode> {
        if rate > self.cpu_clk / 2 {
            return Err(ErrorCode::NOSUPPORT);
        }
        //Divide and truncate
        let mut scaler: u32 = (self.cpu_clk / (2 * rate)) - 1;

        //Increase scaler if the division was not exact, ensuring that it does not overflow
        //or exceed divider specification where tsck is at most <= Tclk/2
        if self.cpu_clk % (2 * rate) != 0 && scaler != 0xFF {
            scaler += 1;
        }
        Ok(scaler as u16)
    }
}

#[derive(Copy, Clone)]
pub struct CS(pub u32);

impl hil::spi::cs::IntoChipSelect<CS, hil::spi::cs::ActiveLow> for CS {
    fn into_cs(self) -> CS {
        self
    }
}

impl<'a> hil::spi::SpiMaster<'a> for SpiHost<'a> {
    type ChipSelect = CS;

    fn init(&self) -> Result<(), ErrorCode> {
        self.err_enable();

        // Disable interrupts explicitly in case they were left enabled by ROM.
        self.disable_interrupts();

        self.enable_spi_host();

        Ok(())
    }

    fn set_client(&self, client: &'a dyn hil::spi::SpiMasterClient) {
        self.client.set(client);
    }

    fn is_busy(&self) -> bool {
        self.busy.get()
    }

    fn read_write_bytes(
        &self,
        tx_buf: SubSliceMut<'static, u8>,
        rx_buf: Option<SubSliceMut<'static, u8>>,
    ) -> Result<
        (),
        (
            ErrorCode,
            SubSliceMut<'static, u8>,
            Option<SubSliceMut<'static, u8>>,
        ),
    > {
        debug_assert!(!self.busy.get());
        debug_assert!(self.tx_buf.is_none());
        debug_assert!(self.rx_buf.is_none());
        let regs = self.registers;

        if self.is_busy() || regs.status.is_set(STATUS::TXFULL) {
            return Err((ErrorCode::BUSY, tx_buf, rx_buf));
        }

        if rx_buf.is_none() {
            return Err((ErrorCode::NOMEM, tx_buf, rx_buf));
        }

        self.tx_len.set(tx_buf.len());

        let mut t_byte: u32;
        let mut tx_slice: [u8; 4];
        //We are committing to the transfer now
        self.set_spi_busy();

        while !regs.status.is_set(STATUS::TXFULL) && regs.status.read(STATUS::TXQD) < 64 {
            tx_slice = [0, 0, 0, 0];
            for elem in tx_slice.iter_mut() {
                if self.tx_offset.get() >= self.tx_len.get() {
                    break;
                }
                *elem = tx_buf[self.tx_offset.get()];
                self.tx_offset.set(self.tx_offset.get() + 1);
            }
            t_byte = u32::from_le_bytes(tx_slice);
            regs.txdata[0].set(t_byte);

            //Transfer Complete in one-shot
            if self.tx_offset.get() >= self.tx_len.get() {
                break;
            }
        }

        //Hold tx_buf for offset transfer continue
        self.tx_buf.replace(tx_buf);

        //Hold rx_buf for later

        if let Some(rx_buf_t) = rx_buf {
            self.rx_len.set(cmp::min(self.tx_len.get(), rx_buf_t.len()));
            self.rx_buf.replace(rx_buf_t);
        }

        //Set command register to init transfer
        self.start_transceive();

        Ok(())
    }

    fn write_byte(&self, _val: u8) -> Result<(), ErrorCode> {
        //Use `read_write_bytes()` instead.
        Err(ErrorCode::FAIL)
    }

    fn read_byte(&self) -> Result<u8, ErrorCode> {
        //Use `read_write_bytes()` instead.
        Err(ErrorCode::FAIL)
    }

    fn read_write_byte(&self, _val: u8) -> Result<u8, ErrorCode> {
        //Use `read_write_bytes()` instead.
        Err(ErrorCode::FAIL)
    }

    fn specify_chip_select(&self, cs: Self::ChipSelect) -> Result<(), ErrorCode> {
        let regs = self.registers;

        //CSID will index the CONFIGOPTS multi-register
        regs.csid.write(CSID::CSID.val(cs.0));

        Ok(())
    }

    fn set_rate(&self, rate: u32) -> Result<u32, ErrorCode> {
        let regs = self.registers;

        match self.calculate_tsck_scaler(rate) {
            Ok(scaler) => {
                regs.configopts[0].modify(CONFIGOPTS::CLKDIV_0.val(scaler as u32));
                self.tsclk.set(rate);
                Ok(rate)
            }
            Err(e) => Err(e),
        }
    }

    fn get_rate(&self) -> u32 {
        self.tsclk.get()
    }

    fn set_polarity(&self, polarity: ClockPolarity) -> Result<(), ErrorCode> {
        let regs = self.registers;
        match polarity {
            ClockPolarity::IdleLow => regs.configopts[0].modify(CONFIGOPTS::CPOL_0::CLEAR),
            ClockPolarity::IdleHigh => regs.configopts[0].modify(CONFIGOPTS::CPOL_0::SET),
        };
        Ok(())
    }

    fn get_polarity(&self) -> ClockPolarity {
        let regs = self.registers;

        match regs.configopts[0].read(CONFIGOPTS::CPOL_0) {
            0 => ClockPolarity::IdleLow,
            1 => ClockPolarity::IdleHigh,
            _ => unreachable!(),
        }
    }

    fn set_phase(&self, phase: ClockPhase) -> Result<(), ErrorCode> {
        let regs = self.registers;
        match phase {
            ClockPhase::SampleLeading => regs.configopts[0].modify(CONFIGOPTS::CPHA_0::CLEAR),
            ClockPhase::SampleTrailing => regs.configopts[0].modify(CONFIGOPTS::CPHA_0::SET),
        };
        Ok(())
    }

    fn get_phase(&self) -> ClockPhase {
        let regs = self.registers;

        match regs.configopts[0].read(CONFIGOPTS::CPHA_0) {
            1 => ClockPhase::SampleTrailing,
            0 => ClockPhase::SampleLeading,
            _ => unreachable!(),
        }
    }

    /// hold_low is controlled by IP based on command segments issued
    /// force holds are not supported
    fn hold_low(&self) {
        unimplemented!("spi_host: does not support hold low");
    }

    /// release_low is controlled by IP based on command segments issued
    /// force releases are not supported
    fn release_low(&self) {
        unimplemented!("spi_host: does not support release low");
    }
}
