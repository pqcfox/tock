// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Support for the AES hardware block on OpenTitan
//!
//! <https://docs.opentitan.org/hw/ip/aes/doc/>

use crate::registers::aes_regs::{AesRegisters, CTRL_SHADOWED, STATUS, TRIGGER};
use core::cell::Cell;
use kernel::deferred_call::{DeferredCall, DeferredCallClient};
use kernel::hil;
use kernel::hil::symmetric_encryption;
use kernel::hil::symmetric_encryption::{AES128_BLOCK_SIZE, AES128_KEY_SIZE};
use kernel::utilities::cells::{OptionalCell, TakeCell};
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::StaticRef;
use kernel::ErrorCode;

const MAX_LENGTH: usize = 128;

#[derive(Clone, Copy)]
enum Mode {
    Idle,
    AES128CTR,
    AES128CBC,
    AES128ECB,
}

pub struct Aes<'a> {
    registers: StaticRef<AesRegisters>,

    client: OptionalCell<&'a dyn hil::symmetric_encryption::Client<'a>>,
    source: TakeCell<'static, [u8]>,
    dest: TakeCell<'static, [u8]>,
    mode: Cell<Mode>,

    deferred_call: DeferredCall,
}

impl<'a> Aes<'a> {
    pub fn new(base: StaticRef<AesRegisters>) -> Aes<'a> {
        Aes {
            registers: base,
            client: OptionalCell::empty(),
            source: TakeCell::empty(),
            dest: TakeCell::empty(),
            mode: Cell::new(Mode::Idle),
            deferred_call: DeferredCall::new(),
        }
    }

    pub fn idle(&self) -> bool {
        self.registers.status.is_set(STATUS::IDLE)
    }

    /// Must wait for IDLE, for trigger to set.
    /// On reset, AES unit will first reseed the internal PRNGs
    /// for register clearing and masking via EDN, and then
    /// clear all key, IV and data registers with pseudo-random data.
    /// Only after this sequence has finished, the unit becomes idle
    ///
    /// NOTE: This is needed for Verilator, and is suggested by documentation
    ///       in general.
    /// Refer: https://docs.opentitan.org/hw/ip/aes/doc/#programmers-guide
    fn wait_on_idle_ready(&self) -> Result<(), ErrorCode> {
        for _i in 0..10000 {
            if self.idle() {
                return Ok(());
            }
        }
        // AES Busy
        Err(ErrorCode::BUSY)
    }

    fn input_ready(&self) -> bool {
        self.registers.status.is_set(STATUS::INPUT_READY)
    }

    /// Wait for the input to be ready, return an error if it takes too long
    fn wait_for_input_ready(&self) -> Result<(), ErrorCode> {
        let mut j = 0;

        while !self.input_ready() {
            j += 1;
            if j > 10000 {
                return Err(ErrorCode::FAIL);
            }
        }

        Ok(())
    }

    fn output_valid(&self) -> bool {
        self.registers.status.is_set(STATUS::OUTPUT_VALID)
    }

    /// Wait for the output to be valid, return an error if it takes too long
    fn wait_for_output_valid(&self) -> Result<(), ErrorCode> {
        let mut j = 0;

        while !self.output_valid() {
            j += 1;
            if j > 10000 {
                return Err(ErrorCode::FAIL);
            }
        }

        Ok(())
    }

    fn read_block(&self, blocknum: usize) -> Result<(), ErrorCode> {
        let blocknum = blocknum * AES128_BLOCK_SIZE;

        self.dest.map_or(Err(ErrorCode::NOMEM), |dest| {
            for i in 0..4 {
                // we work off an array of u8 so we need to assemble those
                // back into a u32
                let mut v = 0;
                match i {
                    0 => v = self.registers.data_out[0].get(),
                    1 => v = self.registers.data_out[1].get(),
                    2 => v = self.registers.data_out[2].get(),
                    3 => v = self.registers.data_out[3].get(),
                    _ => {}
                }
                dest[blocknum + (i * 4)] = v as u8;
                dest[blocknum + (i * 4) + 1] = (v >> 8) as u8;
                dest[blocknum + (i * 4) + 2] = (v >> 16) as u8;
                dest[blocknum + (i * 4) + 3] = (v >> 24) as u8;
            }
            Ok(())
        })
    }

    fn write_block(&self, blocknum: usize) -> Result<(), ErrorCode> {
        self.source.map_or_else(
            || {
                // This is the case that dest = source
                self.dest.map_or(Err(ErrorCode::NOMEM), |dest| {
                    for i in 0..4 {
                        let mut v = dest[blocknum + (i * 4)] as usize;
                        v |= (dest[blocknum + (i * 4) + 1] as usize) << 8;
                        v |= (dest[blocknum + (i * 4) + 2] as usize) << 16;
                        v |= (dest[blocknum + (i * 4) + 3] as usize) << 24;
                        match i {
                            0 => self.registers.data_in[0].set(v as u32),
                            1 => self.registers.data_in[1].set(v as u32),
                            2 => self.registers.data_in[2].set(v as u32),
                            3 => self.registers.data_in[3].set(v as u32),
                            _ => {}
                        }
                    }
                    Ok(())
                })
            },
            |source| {
                for i in 0..4 {
                    // we work off an array of u8 so we need to assemble
                    // those back into a u32
                    let mut v = source[blocknum + (i * 4)] as usize;
                    v |= (source[blocknum + (i * 4) + 1] as usize) << 8;
                    v |= (source[blocknum + (i * 4) + 2] as usize) << 16;
                    v |= (source[blocknum + (i * 4) + 3] as usize) << 24;
                    match i {
                        0 => self.registers.data_in[0].set(v as u32),
                        1 => self.registers.data_in[1].set(v as u32),
                        2 => self.registers.data_in[2].set(v as u32),
                        3 => self.registers.data_in[3].set(v as u32),
                        _ => {}
                    }
                }
                Ok(())
            },
        )
    }

    fn do_crypt(
        &self,
        start_index: usize,
        stop_index: usize,
        mut write_block: usize,
    ) -> Result<(), ErrorCode> {
        let start_block = start_index / AES128_BLOCK_SIZE;
        let end_block = stop_index / AES128_BLOCK_SIZE;

        for i in start_block..end_block {
            self.wait_for_input_ready()?;
            self.write_block(write_block)?;

            self.wait_for_output_valid()?;
            self.read_block(i)?;
            write_block += AES128_BLOCK_SIZE;
        }

        Ok(())
    }
}

impl<'a> hil::symmetric_encryption::AES128<'a> for Aes<'a> {
    fn enable(&self) {
        self.registers.trigger.write(
            TRIGGER::KEY_IV_DATA_IN_CLEAR::SET
                + TRIGGER::DATA_OUT_CLEAR::SET
                + TRIGGER::PRNG_RESEED::SET,
        );
    }

    fn disable(&self) {
        self.registers
            .ctrl_shadowed
            .write(CTRL_SHADOWED::MANUAL_OPERATION::SET);
        self.registers
            .ctrl_shadowed
            .write(CTRL_SHADOWED::MANUAL_OPERATION::SET);

        self.registers
            .ctrl_shadowed
            .write(CTRL_SHADOWED::MANUAL_OPERATION::CLEAR);
        self.registers
            .ctrl_shadowed
            .write(CTRL_SHADOWED::MANUAL_OPERATION::CLEAR);
    }

    fn set_client(&'a self, client: &'a dyn symmetric_encryption::Client<'a>) {
        self.client.set(client);
    }

    fn set_iv(&self, iv: &[u8]) -> Result<(), ErrorCode> {
        self.wait_on_idle_ready()?;

        if iv.len() != AES128_BLOCK_SIZE {
            return Err(ErrorCode::INVAL);
        }

        for i in 0..(AES128_BLOCK_SIZE / 4) {
            let mut k = iv[i * 4] as u32;
            k |= (iv[i * 4 + 1] as u32) << 8;
            k |= (iv[i * 4 + 2] as u32) << 16;
            k |= (iv[i * 4 + 3] as u32) << 24;
            match i {
                0 => self.registers.iv[0].set(k),
                1 => self.registers.iv[1].set(k),
                2 => self.registers.iv[2].set(k),
                3 => self.registers.iv[3].set(k),
                _ => {
                    unreachable!()
                }
            }
        }

        Ok(())
    }

    fn set_key(&self, key: &[u8]) -> Result<(), ErrorCode> {
        self.wait_on_idle_ready()?;

        if key.len() != AES128_KEY_SIZE {
            return Err(ErrorCode::INVAL);
        }

        for i in 0..(AES128_KEY_SIZE / 4) {
            let mut k = key[i * 4] as u32;
            k |= (key[i * 4 + 1] as u32) << 8;
            k |= (key[i * 4 + 2] as u32) << 16;
            k |= (key[i * 4 + 3] as u32) << 24;
            match i {
                0 => {
                    self.registers.key_share0[0].set(k);
                    self.registers.key_share1[0].set(0);
                }
                1 => {
                    self.registers.key_share0[1].set(k);
                    self.registers.key_share1[1].set(0);
                }
                2 => {
                    self.registers.key_share0[2].set(k);
                    self.registers.key_share1[2].set(0);
                }
                3 => {
                    self.registers.key_share0[3].set(k);
                    self.registers.key_share1[3].set(0);
                }
                _ => {
                    unreachable!()
                }
            }
        }

        // We must write the rest of the registers as well
        // This should be written with random data, for now this will do
        self.registers.key_share0[4].set(0x12);
        self.registers.key_share0[5].set(0x34);
        self.registers.key_share0[6].set(0x56);
        self.registers.key_share0[7].set(0x78);

        self.registers.key_share1[4].set(0xAB);
        self.registers.key_share1[5].set(0xCD);
        self.registers.key_share1[6].set(0xEF);
        self.registers.key_share1[7].set(0x00);

        Ok(())
    }

    fn start_message(&self) {}

    fn crypt(
        &self,
        source: Option<&'static mut [u8]>,
        dest: &'static mut [u8],
        start_index: usize,
        stop_index: usize,
    ) -> Option<(
        Result<(), ErrorCode>,
        Option<&'static mut [u8]>,
        &'static mut [u8],
    )> {
        match stop_index.checked_sub(start_index) {
            None => return Some((Err(ErrorCode::INVAL), source, dest)),
            Some(s) => {
                if s > MAX_LENGTH {
                    return Some((Err(ErrorCode::INVAL), source, dest));
                }
                if s % AES128_BLOCK_SIZE != 0 {
                    return Some((Err(ErrorCode::INVAL), source, dest));
                }
            }
        }

        if self.deferred_call.is_pending() {
            return Some((
                Err(ErrorCode::BUSY),
                self.source.take(),
                self.dest.take().unwrap(),
            ));
        }

        self.dest.replace(dest);
        let ret = match source {
            None => self.do_crypt(start_index, stop_index, start_index),
            Some(src) => {
                self.source.replace(src);
                self.do_crypt(start_index, stop_index, 0)
            }
        };

        if ret.is_ok() {
            // Schedule a deferred call
            self.deferred_call.set();
            None
        } else {
            Some((ret, self.source.take(), self.dest.take().unwrap()))
        }
    }
}

impl kernel::hil::symmetric_encryption::AES128Ctr for Aes<'_> {
    fn set_mode_aes128ctr(&self, encrypting: bool) -> Result<(), ErrorCode> {
        self.wait_on_idle_ready()?;
        self.mode.set(Mode::AES128CTR);

        let mut ctrl = if encrypting {
            CTRL_SHADOWED::OPERATION::AES_ENC
        } else {
            CTRL_SHADOWED::OPERATION::AES_DEC
        };
        ctrl += CTRL_SHADOWED::MODE::AES_CTR;
        // Tock only supports 128-bit keys
        ctrl += CTRL_SHADOWED::KEY_LEN::AES_128;
        ctrl += CTRL_SHADOWED::MANUAL_OPERATION::CLEAR;

        // We need to set the control register twice as it's shadowed
        self.registers.ctrl_shadowed.write(ctrl);
        self.registers.ctrl_shadowed.write(ctrl);

        Ok(())
    }
}

impl kernel::hil::symmetric_encryption::AES128ECB for Aes<'_> {
    fn set_mode_aes128ecb(&self, encrypting: bool) -> Result<(), ErrorCode> {
        self.wait_on_idle_ready()?;
        self.mode.set(Mode::AES128ECB);

        let mut ctrl = if encrypting {
            CTRL_SHADOWED::OPERATION::AES_ENC
        } else {
            CTRL_SHADOWED::OPERATION::AES_DEC
        };
        ctrl += CTRL_SHADOWED::MODE::AES_ECB;
        // Tock only supports 128-bit keys
        ctrl += CTRL_SHADOWED::KEY_LEN::AES_128;
        ctrl += CTRL_SHADOWED::MANUAL_OPERATION::CLEAR;

        // We need to set the control register twice as it's shadowed
        self.registers.ctrl_shadowed.write(ctrl);
        self.registers.ctrl_shadowed.write(ctrl);

        Ok(())
    }
}

impl kernel::hil::symmetric_encryption::AES128CBC for Aes<'_> {
    fn set_mode_aes128cbc(&self, encrypting: bool) -> Result<(), ErrorCode> {
        self.wait_on_idle_ready()?;
        self.mode.set(Mode::AES128CBC);

        let mut ctrl = if encrypting {
            CTRL_SHADOWED::OPERATION::AES_ENC
        } else {
            CTRL_SHADOWED::OPERATION::AES_DEC
        };
        ctrl += CTRL_SHADOWED::MODE::AES_CBC;
        // Tock only supports 128-bit keys
        ctrl += CTRL_SHADOWED::KEY_LEN::AES_128;
        ctrl += CTRL_SHADOWED::MANUAL_OPERATION::CLEAR;

        // We need to set the control register twice as it's shadowed
        self.registers.ctrl_shadowed.write(ctrl);
        self.registers.ctrl_shadowed.write(ctrl);

        Ok(())
    }
}

impl DeferredCallClient for Aes<'_> {
    fn register(&'static self) {
        self.deferred_call.register(self);
    }

    fn handle_deferred_call(&self) {
        self.client.map(|client| {
            client.crypt_done(self.source.take(), self.dest.take().unwrap());
        });
    }
}
