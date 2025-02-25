// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! OpenTitan SPI device driver (stub)

use crate::registers::spi_device_regs::{SpiDeviceRegisters, INTR};
use kernel::utilities::registers::interfaces::ReadWriteable;
use kernel::utilities::StaticRef;

// TODO: Implement this

pub struct SpiDevice<'a> {
    base: StaticRef<SpiDeviceRegisters>,
    _temp: core::marker::PhantomData<&'a ()>,
}

#[derive(Clone, Copy)]
pub enum SpiDeviceInterrupt {
    /// Upload Command FIFO is not empty
    UploadCmdfifoNotEmpty,
    /// Upload payload is not empty. The event occurs after SPI transaction
    /// completed
    UploadPayloadNotEmpty,
    /// Upload payload overflow event. When a SPI Host system issues a command
    /// with payload more than 256B, this event is reported. When it happens, SW
    /// should read the last written payload index CSR to figure out the
    /// starting address of the last 256B.
    UploadPayloadOverflow,
    /// Read Buffer Threshold event. The host system accesses greater than or
    /// equal to the threshold of a buffer.
    ReadbufWatermark,
    /// Read buffer flipped event. The host system accesses other side of
    /// buffer.
    ReadbufFlip,
    /// TPM Header(Command/Address) buffer available
    TpmHeaderNotEmpty,
    /// TPM RdFIFO command ended. The TPM Read command targeting the RdFIFO
    /// ended. Check TPM_STATUS.rdfifo_aborted to see if the transaction
    /// completed.
    TpmRdfifoCmdEnd,
    /// TPM RdFIFO data dropped. Data was dropped from the RdFIFO. Data was
    /// written while a read command was not active, and it was not
    /// accepted. This can occur when the host aborts a read command.
    TpmRdfifoDrop,
}

impl<'a> SpiDevice<'a> {
    /// Constructs a new SPI device driver.
    pub fn new(base: StaticRef<SpiDeviceRegisters>) -> SpiDevice<'a> {
        SpiDevice {
            base,
            _temp: core::marker::PhantomData,
        }
    }

    /// Handler for SPI Device interrupts.
    pub fn handle_interrupt(&self, interrupt: SpiDeviceInterrupt) {
        match interrupt {
            SpiDeviceInterrupt::UploadCmdfifoNotEmpty => {
                self.base
                    .intr_state
                    .modify(INTR::UPLOAD_CMDFIFO_NOT_EMPTY::SET);
                // TODO: handle this interrupt
            }
            SpiDeviceInterrupt::UploadPayloadNotEmpty => {
                self.base
                    .intr_state
                    .modify(INTR::UPLOAD_PAYLOAD_NOT_EMPTY::SET);
                // TODO: handle this interrupt
            }
            SpiDeviceInterrupt::UploadPayloadOverflow => {
                self.base
                    .intr_state
                    .modify(INTR::UPLOAD_PAYLOAD_OVERFLOW::SET);
                // TODO: handle this interrupt
            }
            SpiDeviceInterrupt::ReadbufWatermark => {
                self.base.intr_state.modify(INTR::READBUF_WATERMARK::SET);
                // TODO: handle this interrupt
            }
            SpiDeviceInterrupt::ReadbufFlip => {
                self.base.intr_state.modify(INTR::READBUF_FLIP::SET);
                // TODO: handle this interrupt
            }
            SpiDeviceInterrupt::TpmHeaderNotEmpty => {
                self.base.intr_state.modify(INTR::TPM_HEADER_NOT_EMPTY::SET);
                // TODO: handle this interrupt
            }
            SpiDeviceInterrupt::TpmRdfifoCmdEnd => {
                self.base.intr_state.modify(INTR::TPM_RDFIFO_CMD_END::SET);
                // TODO: handle this interrupt
            }
            SpiDeviceInterrupt::TpmRdfifoDrop => {
                self.base.intr_state.modify(INTR::TPM_RDFIFO_DROP::SET);
                // TODO: handle this interrupt
            }
        }
    }
}
