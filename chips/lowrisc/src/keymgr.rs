// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! OpenTitan Key Manager driver (stub)

use crate::registers::keymgr_regs::{KeymgrRegisters, INTR};
use kernel::utilities::registers::interfaces::ReadWriteable;
use kernel::utilities::StaticRef;

// TODO: Implement this

pub struct Keymgr<'a> {
    base: StaticRef<KeymgrRegisters>,
    _temp: core::marker::PhantomData<&'a ()>,
}

#[derive(Clone, Copy)]
pub enum KeymgrInterrupt {
    /// Operation complete
    OpDone,
}

impl<'a> Keymgr<'a> {
    /// Constructs a new Key Manager driver.
    pub fn new(base: StaticRef<KeymgrRegisters>) -> Keymgr<'a> {
        Keymgr {
            base,
            _temp: core::marker::PhantomData,
        }
    }

    /// Handler for Key Manager interrupts.
    pub fn handle_interrupt(&self, interrupt: KeymgrInterrupt) {
        match interrupt {
            KeymgrInterrupt::OpDone => {
                self.base.intr_state.modify(INTR::OP_DONE::SET);
            }
        }
    }
}
