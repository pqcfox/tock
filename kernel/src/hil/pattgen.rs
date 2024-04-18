// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Interfaces for Pattern Generator output.

use crate::ErrorCode;

use core::num::NonZeroUsize;

/// Pattern generator control
pub trait PattGen {
    type Channel: TryFrom<usize>;
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
}
