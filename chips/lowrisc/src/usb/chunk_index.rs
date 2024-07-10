// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Chunk index

use super::buffer::BUFFER_SIZE;
use super::buffer_index::BufferIndex;
use super::utils;

use core::num::NonZeroUsize;

/// The size of a chunk in bytes
pub(super) const CHUNK_SIZE: NonZeroUsize =
    utils::create_non_zero_usize(core::mem::size_of::<u32>());
/// Number of chunks per buffer
pub(super) const NUMBER_CHUNKS_PER_BUFFER: NonZeroUsize =
    utils::create_non_zero_usize(BUFFER_SIZE.get() / CHUNK_SIZE.get());
/// Number of chunks in controller's buffer
pub(super) const NUMBER_CHUNKS: NonZeroUsize = utils::create_non_zero_usize(512);

/// Index of a chunk
#[derive(Clone, Copy, Debug)]
pub(super) struct ChunkIndex(usize);

impl ChunkIndex {
    /// Returns the index of the first chunk in the given buffer
    ///
    /// # Parameters
    ///
    /// + `buffer_index`: the index of the buffer
    ///
    /// # Return value
    ///
    /// The first index of the given buffer
    pub(super) const fn new_from_buffer_index(buffer_index: BufferIndex) -> Self {
        Self(buffer_index.to_usize() * NUMBER_CHUNKS_PER_BUFFER.get())
    }

    /// Returns the next chunk index, if any
    ///
    /// # Return value
    ///
    /// + Some: the next chunk index
    /// + None: `self` is the last chunk index
    pub(super) const fn next(self) -> Option<Self> {
        let maybe_next = self.0 + 1;
        if maybe_next < NUMBER_CHUNKS.get() {
            Some(Self(maybe_next))
        } else {
            None
        }
    }

    /// Converts the chunk index to usize
    ///
    /// # Return value
    ///
    /// The usize representation of `self`
    pub(super) const fn to_usize(self) -> usize {
        self.0
    }
}
