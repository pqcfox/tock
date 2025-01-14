// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

/// Redirect polyfill for 64-bit division to `udiv64_slow`, because the ROM math library
/// black-holes the libgcc-intrinsic `__udivdi3`, which is called internally by LLVM.
#[no_mangle]
pub extern "C" fn __udivdi3(dividend: u64, divisor: u64) -> u64 {
    // SAFETY: `udiv64_slow` imposes no constraints on the values of its arguments. Passing `NULL`
    // as `rem_out` causes it to be ignored, and the API promises not to fault if `divisor == 0`.
    unsafe { otbindgen::udiv64_slow(dividend, divisor, core::ptr::null_mut()) }
}
