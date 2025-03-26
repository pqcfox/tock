// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

//! Buffer index

use core::num::NonZeroUsize;

/// Number of buffers
pub(super) const NUMBER_BUFFERS: NonZeroUsize = match NonZeroUsize::new(32) {
    Some(non_zero_usize) => non_zero_usize,
    None => unreachable!(),
};

#[derive(Clone, Copy, Debug)]
#[rustfmt::skip]
/// List of all buffer indices
pub(super) struct BufferIndex(u8);

impl BufferIndex {
    /// Tries to create a new buffer index from the given usize
    ///
    /// # Parameters
    ///
    /// + `value`: the usize supposed to represent a buffer index
    ///
    /// # Return value
    ///
    /// + Ok: value is valid (< 32)
    /// + Err: value is invalid (>= 32)
    pub(super) const fn try_from_usize(value: usize) -> Result<Self, ()> {
        if value >= NUMBER_BUFFERS.get() {
            Err(())
        } else {
            // CAST: Because of the if condition, value < NUMBER_BUFFERS = 32, so value fits in a
            // u8
            Ok(Self(value as u8))
        }
    }

    /// Converts the given buffer index to a usize.
    ///
    /// # Return value
    ///
    /// The usize representation of the buffer index.
    pub(super) const fn to_usize(self) -> usize {
        // CAST: size_of(usize) > size_of(u8) on RV32I
        self.0 as usize
    }

    /// Returns the buffer index following this buffer index, wrapping if this buffer index is the
    /// last buffer index.
    ///
    /// # Return value
    ///
    /// The buffer index that follows `self`.
    pub(super) const fn next_wrapping(self) -> Self {
        // CAST: NUMBER_BUFFERS == 32 which fits in u8
        Self((self.0 + 1) % NUMBER_BUFFERS.get() as u8)
    }
}

/// Buffer index 0
pub(super) const BUFFER_INDEX_0: BufferIndex = BufferIndex(0);
