// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

/// All possible FIFO levels.
#[repr(u8)]
pub(super) enum FifoLevel {
    Level0,
    #[allow(unused)]
    Level1,
    #[allow(unused)]
    Level2,
    #[allow(unused)]
    Level3,
    #[allow(unused)]
    Level4,
    #[allow(unused)]
    Level5,
    #[allow(unused)]
    Level6,
    #[allow(unused)]
    Level7,
    #[allow(unused)]
    Level8,
    #[allow(unused)]
    Level9,
    #[allow(unused)]
    Level10,
    #[allow(unused)]
    Level11,
    #[allow(unused)]
    Level12,
    #[allow(unused)]
    Level13,
    #[allow(unused)]
    Level14,
    Level15,
    Level16,
}

impl FifoLevel {
    /// Convert the enum to the underlying integer type
    pub(super) const fn inner(self) -> u8 {
        // The enum is marked as u8, so the cast is safe.
        self as u8
    }
}
