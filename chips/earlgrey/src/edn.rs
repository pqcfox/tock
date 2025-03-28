// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::registers::edn_regs::EdnRegisters;
use crate::registers::top_earlgrey::{EDN0_BASE_ADDR, EDN1_BASE_ADDR};
use kernel::utilities::StaticRef;

pub const EDN0_BASE: StaticRef<EdnRegisters> =
    unsafe { StaticRef::new(EDN0_BASE_ADDR as *const EdnRegisters) };

pub const EDN1_BASE: StaticRef<EdnRegisters> =
    unsafe { StaticRef::new(EDN1_BASE_ADDR as *const EdnRegisters) };
