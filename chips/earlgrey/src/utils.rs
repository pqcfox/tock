// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use core::num::NonZeroU8;
use core::num::NonZeroUsize;

pub(crate) const fn create_non_zero_usize(value: usize) -> NonZeroUsize {
    match NonZeroUsize::new(value) {
        Some(non_zero_usize) => non_zero_usize,
        None => panic!("Attempting to create invalid NonZeroUsize"),
    }
}

pub(crate) const fn create_non_zero_u8(value: u8) -> NonZeroU8 {
    match NonZeroU8::new(value) {
        Some(non_zero_u8) => non_zero_u8,
        None => panic!("Attempting to create invalid NonZeroU8"),
    }
}
