// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

//! Buffer

use super::utils;

use core::cell::Cell;
use core::num::NonZeroUsize;

/// A hardware buffer.
///
/// The USB driver has a collection of buffers used to store data to be transmitted or to be
/// received.
pub(super) struct Buffer {
    available: Cell<bool>,
}

impl Buffer {
    /// Check if the buffer is available.
    ///
    /// A hardware buffer is available if not used by any endpoint.
    ///
    /// # Return value
    ///
    /// + false: the buffer is not available, i.e. occupied
    /// + true: the buffer is available, i.e. free
    pub(super) fn is_available(&self) -> bool {
        self.available.get()
    }

    /// Frees the buffer.
    ///
    /// Frees the buffer, marking it available.
    pub(super) fn free(&self) {
        self.available.set(true);
    }

    /// Occupies the buffer.
    ///
    /// Occupies the buffer, marking it unavailable.
    pub(super) fn occupy(&self) {
        self.available.set(false);
    }
}

/// Available buffer.
#[allow(clippy::declare_interior_mutable_const)]
pub(super) const AVAILABLE_BUFFER: Buffer = Buffer {
    available: Cell::new(true),
};

/// Number of hardware buffers
pub(super) const BUFFER_SIZE: NonZeroUsize = utils::create_non_zero_usize(64);
