// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use kernel::utilities::StaticRef;
pub use lowrisc::keymgr::Keymgr;
use lowrisc::registers::keymgr_regs::KeymgrRegisters;

use crate::registers::top_earlgrey::KEYMGR_BASE_ADDR;

pub const KEYMGR_BASE: StaticRef<KeymgrRegisters> =
    unsafe { StaticRef::new(KEYMGR_BASE_ADDR as *const KeymgrRegisters) };
