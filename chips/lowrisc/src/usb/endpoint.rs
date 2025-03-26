// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

//! USB endpoint

use super::endpoint_state::{CtrlEndpointState, EndpointState, ReceiveCtrlEndpointState};

use kernel::utilities::cells::{OptionalCell, VolatileCell};

use core::cell::Cell;

/// USB endpoint
///
/// A USB endpoint is a collection of:
///
/// + a user buffer used for IN transactions
/// + a user buffer used for OUT and SETUP transactions
pub(super) struct Endpoint<'a> {
    buffer_in: OptionalCell<&'a [VolatileCell<u8>]>,
    buffer_out: OptionalCell<&'a [VolatileCell<u8>]>,
    state: Cell<EndpointState>,
    // Indicates whether buffer_in contains the last packet to be transmitted as part of a long
    // transaction
    last: Cell<bool>,
}

impl<'a> Endpoint<'a> {
    /// Creates a new USB endpoint.
    ///
    /// The new USB endpoint lacks buffers. They need to be provided by future calls to
    /// [set_buffer_in()] and [set_buffer_out()]. The endpoint is configured by default as a
    /// control endpoint.
    ///
    /// # Return value
    ///
    /// A new instance of [Endpoint]
    pub(super) const fn new() -> Self {
        Self {
            buffer_in: OptionalCell::empty(),
            buffer_out: OptionalCell::empty(),
            state: Cell::new(EndpointState::Ctrl(CtrlEndpointState::Receive(
                ReceiveCtrlEndpointState::Setup,
            ))),
            last: Cell::new(false),
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

    /// Return the IN buffer.
    ///
    /// # Return value
    ///
    /// + Some: the IN buffer
    /// + None: the endpoint lacks an IN buffer
    pub(super) fn get_buffer_in(&self) -> &OptionalCell<&'a [VolatileCell<u8>]> {
        &self.buffer_in
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

    /// Return the OUT buffer.
    ///
    /// # Return value
    ///
    /// + Some: the OUT buffer
    /// + None: the endpoint lacks an OUT buffer
    pub(super) fn get_buffer_out(&self) -> &OptionalCell<&'a [VolatileCell<u8>]> {
        &self.buffer_out
    }

    /// Returns the state of the endpoint
    ///
    /// # Return value
    ///
    /// Endpoint's state
    pub(super) fn get_state(&self) -> EndpointState {
        self.state.get()
    }

    /// Sets the state of the endpoint
    ///
    /// # Parameters
    ///
    /// + ̀`state`: the state to be set
    pub(super) fn set_state(&self, state: EndpointState) {
        self.state.set(state);
    }

    /// Returns true if `buffer_in` contains the last packet to be transmitted as part of a long
    /// transaction, false otherwise.
    ///
    /// # Return value
    ///
    /// + false: there are still other packets that need to be transmitted
    /// + true: `buffer_in` contains the last packet to be transmitted
    pub(super) fn get_last(&self) -> bool {
        self.last.get()
    }

    /// Sets `last` packet to be transmitted mark
    ///
    /// # Parameters
    ///
    /// + `last`: indicate whether `buffer_in` contains the last packet to be transmitted
    pub(super) fn set_last(&self, last: bool) {
        self.last.set(last)
    }
}
