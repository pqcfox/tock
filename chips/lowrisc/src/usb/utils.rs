// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

//! Utilities used across the USB driver

use core::num::{NonZeroU8, NonZeroUsize};

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

/// Creates a new NonZeroU8
///
/// # Parameters
///
/// + `value`: the value to be converted to NonZeroU8
///
/// # Return value
///
/// The NonZeroU8 representation of `value`
///
/// # Panic
///
/// This function panics if value == 0.
#[track_caller]
pub(super) const fn create_non_zero_u8(value: u8) -> NonZeroU8 {
    match NonZeroU8::new(value) {
        Some(non_zero_u8) => non_zero_u8,
        None => panic!("Attempt to create invalid NonZeroU8"),
    }
}
