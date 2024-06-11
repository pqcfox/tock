// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Utilities used across the USB driver

use core::num::NonZeroUsize;

/// Creates a new NonZeroUsize
///
/// # Parameters
///
/// + `value`: the value to be converted to NonZeroUsize
///
/// # Return value
///
/// The NonZeroUsize representation of `value`
///
/// # Panic
///
/// This function panics if value == 0.
#[track_caller]
pub(super) const fn create_non_zero_usize(value: usize) -> NonZeroUsize {
    match NonZeroUsize::new(value) {
        Some(non_zero_usize) => non_zero_usize,
        None => panic!("Attempt to create invalid NonZeroUsize"),
    }
}
