// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

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
