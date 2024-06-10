// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! USB endpoint

use kernel::utilities::cells::{OptionalCell, VolatileCell};

/// USB endpoint
///
/// A USB endpoint is a collection of:
///
/// + a user buffer used for IN transactions
/// + a user buffer used for OUT and SETUP transactions
pub(super) struct Endpoint<'a> {
    buffer_in: OptionalCell<&'a [VolatileCell<u8>]>,
    buffer_out: OptionalCell<&'a [VolatileCell<u8>]>,
}

impl<'a> Endpoint<'a> {
    /// Creates a new USB endpoint.
    ///
    /// The new USB endpoint lacks buffers. They need to be provided by future calls to
    /// [set_buffer_in()] and [set_buffer_out()].
    ///
    /// # Return value
    ///
    /// A new instance of [Endpoint]
    pub(super) const fn new() -> Self {
        Self {
            buffer_in: OptionalCell::empty(),
            buffer_out: OptionalCell::empty(),
        }
    }

    /// Sets the IN buffer.
    ///
    /// Sets the IN buffer to be used by future IN transactions.
    ///
    /// # Parameters
    ///
    /// + `buffer_in`: the IN buffer to be used by the endpoint
    pub(super) fn set_buffer_in(&self, buffer_in: &'a [VolatileCell<u8>]) {
        self.buffer_in.set(buffer_in);
    }

    /// Sets the OUT buffer.
    ///
    /// Sets the OUT buffer to be used by future OUT transactions.
    ///
    /// # Parameters
    ///
    /// + `buffer_out`: the OUT buffer to be used by the endpoint
    pub(super) fn set_buffer_out(&self, buffer_out: &'a [VolatileCell<u8>]) {
        self.buffer_out.set(buffer_out);
    }
}
