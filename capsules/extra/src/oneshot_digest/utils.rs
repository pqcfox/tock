// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

//! Utilities for oneshot digest capsule operations.

use kernel::grant::GrantKernelData;
use kernel::processbuffer::{
    ReadOnlyProcessBufferRef, ReadWriteProcessBufferRef, ReadableProcessBuffer,
    WriteableProcessBuffer,
};
use kernel::ErrorCode;

/// Helper that executes a closure using a read-only allow if it exists,
/// with an optional length check.
pub fn require_ro_buffer<T>(
    grant: &GrantKernelData<'_>,
    allow_id: usize,
    required_length: Option<usize>,
    f: impl Fn(ReadOnlyProcessBufferRef<'_>) -> Result<T, ErrorCode>,
) -> Result<T, ErrorCode> {
    let buffer = grant
        .get_readonly_processbuffer(allow_id)
        .map_err(|_| ErrorCode::RESERVE)?;
    if let Some(req) = required_length {
        if req != buffer.len() {
            return Err(ErrorCode::SIZE);
        }
    }
    f(buffer)
}

/// Helper that executes a closure using a read-only allow if it exists,
/// with an optional length check.
pub fn require_rw_buffer<T>(
    grant: &GrantKernelData<'_>,
    allow_id: usize,
    required_length: Option<usize>,
    f: impl Fn(ReadWriteProcessBufferRef<'_>) -> Result<T, ErrorCode>,
) -> Result<T, ErrorCode> {
    let buffer = grant
        .get_readwrite_processbuffer(allow_id)
        .map_err(|_| ErrorCode::RESERVE)?;
    if let Some(req) = required_length {
        if req != buffer.len() {
            return Err(ErrorCode::SIZE);
        }
    }
    f(buffer)
}

/// Helper function that copies a stack-allocated buffer to a grant-allocated buffer.
pub fn copy_digest(src: &[u32], dest: &ReadWriteProcessBufferRef<'_>) -> Result<(), ErrorCode> {
    dest.mut_enter(|dest| {
        let mut chunks = dest.chunks(core::mem::size_of::<u32>());
        for val in src {
            let chunk = match chunks.next() {
                Some(chunk) => chunk,
                // Should never happen, fixed width is checked prior to digest operation
                None => return Err(ErrorCode::SIZE),
            };
            chunk.copy_from_slice(&val.to_ne_bytes())
        }
        Ok(())
    })
    .unwrap_or(Err(ErrorCode::RESERVE))
}
