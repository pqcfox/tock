// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use super::endpoint_index::EndpointIndex;

pub(super) struct EndpointIndexIterator(Option<EndpointIndex>);

impl EndpointIndexIterator {
    pub(super) const fn new() -> Self {
        Self(Some(EndpointIndex::Endpoint0))
    }
}

impl Iterator for EndpointIndexIterator {
    type Item = EndpointIndex;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.0;

        self.0 = match self.0 {
            Some(endpoint_index) => match endpoint_index {
                EndpointIndex::Endpoint0 => Some(EndpointIndex::Endpoint1),
                EndpointIndex::Endpoint1 => Some(EndpointIndex::Endpoint2),
                EndpointIndex::Endpoint2 => Some(EndpointIndex::Endpoint3),
                EndpointIndex::Endpoint3 => Some(EndpointIndex::Endpoint4),
                EndpointIndex::Endpoint4 => Some(EndpointIndex::Endpoint5),
                EndpointIndex::Endpoint5 => Some(EndpointIndex::Endpoint6),
                EndpointIndex::Endpoint6 => Some(EndpointIndex::Endpoint7),
                EndpointIndex::Endpoint7 => Some(EndpointIndex::Endpoint8),
                EndpointIndex::Endpoint8 => Some(EndpointIndex::Endpoint9),
                EndpointIndex::Endpoint9 => Some(EndpointIndex::Endpoint10),
                EndpointIndex::Endpoint10 => Some(EndpointIndex::Endpoint11),
                EndpointIndex::Endpoint11 => None,
            },
            None => None,
        };

        result
    }
}
