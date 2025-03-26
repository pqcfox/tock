// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

//! Interfaces for Pattern Generator output.

use crate::ErrorCode;

use core::num::NonZeroUsize;

/// Pattern generator control
pub trait PattGen<'a> {
    type Channel: TryFrom<usize> + Into<usize>;
    type PatternLength: TryFrom<NonZeroUsize>;
    type PatternRepetitionCount: TryFrom<NonZeroUsize>;

    /// Start a pattern on the given channel
    fn start(
        &self,
        pattern: &[u32; 2],
        pattern_length: Self::PatternLength,
        pattern_repetition_count: Self::PatternRepetitionCount,
        predivider: usize,
        channel: Self::Channel,
    ) -> Result<(), ErrorCode>;

    /// Stop the given channel
    fn stop(&self, channel: Self::Channel) -> Result<(), ErrorCode>;

    /// Set a client to receive callbacks
    fn set_client(&self, client: &'a dyn PattGenClient<Self::Channel>);
}

/// Pattern generator client
pub trait PattGenClient<Channel: Into<usize>> {
    /// Callback when pattern generation finished on a given channel
    fn pattgen_done(&self, channel: Channel);
}
