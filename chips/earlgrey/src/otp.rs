// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use kernel::utilities::StaticRef;
pub use lowrisc::otp::Otp;
use lowrisc::registers::otp_ctrl_regs::OtpCtrlRegisters;

use crate::registers::top_earlgrey::OTP_CTRL_CORE_BASE_ADDR;

pub const OTP_BASE: StaticRef<OtpCtrlRegisters> =
    unsafe { StaticRef::new(OTP_CTRL_CORE_BASE_ADDR as *const OtpCtrlRegisters) };
