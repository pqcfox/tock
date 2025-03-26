// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Packet size

use super::utils;

use core::num::NonZeroUsize;

/// The maximum size of a packet
pub const MAXIMUM_PACKET_SIZE: NonZeroUsize = utils::create_non_zero_usize(64);

/// Size of a packet
#[derive(Clone, Copy, Debug)]
pub(super) struct PacketSize(usize);

impl PacketSize {
    /// Tries to create packet size from the given usize
    ///
    /// # Parameters
    ///
    /// + `value`: the usize to be converted to the size of a packet
    ///
    /// # Return value
    ///
    /// + Ok: value is valid (<= [MAXIMUM_PACKET_SIZE])
    /// + Err: value is invalid (> [MAXIMUM_PACKET_SIZE])
    pub(super) const fn try_from_usize(value: usize) -> Result<Self, ()> {
        if value > MAXIMUM_PACKET_SIZE.get() {
            Err(())
        } else {
            Ok(Self(value))
        }
    }

    /// Converts the packet size to a usize
    ///
    /// # Return value
    ///
    /// The usize representation of self
    pub(super) const fn to_usize(self) -> usize {
        self.0
    }
}

/// Size of an empty packet
pub(super) const EMPTY_PACKET_SIZE: PacketSize = match PacketSize::try_from_usize(0) {
    Ok(packet_size) => packet_size,
    Err(()) => unreachable!(),
};
