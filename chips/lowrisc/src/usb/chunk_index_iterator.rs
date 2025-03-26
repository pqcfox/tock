// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Iterator over the hardware chunks of a buffer.

use super::buffer_index::BufferIndex;
use super::chunk_index::{ChunkIndex, NUMBER_CHUNKS_PER_BUFFER};

/// Iterator over the chunks of a buffer
pub(super) struct ChunkIndexIterator {
    current_chunk_index: Option<ChunkIndex>,
    remaining_size: usize,
}

impl ChunkIndexIterator {
    /// [ChunkIndexIterator] constructor
    ///
    /// # Parameters
    ///
    /// + `buffer_index`: the index of the buffer that must be iterated over
    ///
    /// # Return value
    ///
    /// A new instance of [ChunkIndexIterator]
    pub(super) const fn new(buffer_index: BufferIndex) -> Self {
        Self {
            current_chunk_index: Some(ChunkIndex::new_from_buffer_index(buffer_index)),
            remaining_size: NUMBER_CHUNKS_PER_BUFFER.get(),
        }
    }
}

impl core::iter::Iterator for ChunkIndexIterator {
    type Item = ChunkIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_size != 0 {
            self.remaining_size -= 1;
            let current_chunk_index = self.current_chunk_index;
            self.current_chunk_index = self
                .current_chunk_index
                .and_then(|current_chunk_index| current_chunk_index.next());
            current_chunk_index
        } else {
            None
        }
    }
}
