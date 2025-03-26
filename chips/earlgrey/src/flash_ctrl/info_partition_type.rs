// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

/// All info partition types.
///
/// Unlike data partitions, there can be multiple info partitions. This enum lists all existing
/// info partitions.
#[derive(Clone, Copy, Debug)]
pub enum InfoPartitionType {
    Type0,
    Type1,
    Type2,
}

impl TryFrom<usize> for InfoPartitionType {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InfoPartitionType::Type0),
            1 => Ok(InfoPartitionType::Type1),
            2 => Ok(InfoPartitionType::Type2),
            _ => Err(()),
        }
    }
}
