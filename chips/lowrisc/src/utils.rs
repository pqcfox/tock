// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use core::num::NonZeroUsize;

pub(crate) const fn create_non_zero_usize(value: usize) -> NonZeroUsize {
    match NonZeroUsize::new(value) {
        None => panic!("Attempted to create NonZeroUsize with 0 as value"),
        Some(non_zero_value) => non_zero_value,
    }
}
