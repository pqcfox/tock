// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Endpoint states.

/// Control endpoint waiting for a receive packet
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReceiveCtrlEndpointState {
    /// The control endpoint waits for a SETUP packet
    Setup,
    /// The control endpoint waits for an OUT packet in data stage
    Data,
    /// The control endpoint waits for an OUT packet in status stage
    Status,
}

/// Control endpoint waiting for a packet to be transmitted
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TransmitCtrlEndpointState {
    /// The control endpoint waits for an IN packet to be transmitted in data stage
    Data,
    /// The control endpoint waits for an IN packet to be transmitted in status stage
    Status,
}

/// Endpoint configured as a control endpoint
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CtrlEndpointState {
    /// The endpoint waits to receive a packet
    Receive(ReceiveCtrlEndpointState),
    /// The endpoint waits for an IN packet to be transmitted
    Transmit(TransmitCtrlEndpointState),
}

/// Endpoint state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EndpointState {
    /// The endpoint is configured as a control endpoint
    Ctrl(CtrlEndpointState),
    /// The endpoint is configured as a bulk endpoint.
    Bulk,
    /// The endpoint is configured as an interrupt endpoint.
    Interrupt,
    /// The endpoint is configured as an isochronous endpoint.
    Isochronous,
}
