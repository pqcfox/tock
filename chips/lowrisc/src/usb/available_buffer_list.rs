// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

//! Available buffer list

use super::buffer::{Buffer, AVAILABLE_BUFFER};
use super::buffer_index::{BufferIndex, BUFFER_INDEX_0, NUMBER_BUFFERS};

use core::cell::Cell;

/// Available buffer list.
///
/// This structure keeps track of the available buffers, i.e. a buffer not used by any endpoint.
pub(super) struct AvailableBufferList {
    buffers: [Buffer; NUMBER_BUFFERS.get()],
    current_buffer_index: Cell<BufferIndex>,
}

impl AvailableBufferList {
    /// Available buffer constructor.
    ///
    /// Create a new list of available buffers as a circular list. Initially, all buffers are
    /// marked as available.
    ///
    /// # Return value
    ///
    /// A new instance of [AvailableBufferList].
    pub(super) const fn new() -> Self {
        Self {
            buffers: [AVAILABLE_BUFFER; NUMBER_BUFFERS.get()],
            current_buffer_index: Cell::new(BUFFER_INDEX_0),
        }
    }

    /// Get a buffer.
    ///
    /// Returns a reference to the `buffer_index`th buffer.
    ///
    /// # Parameters
    ///
    /// + `buffer_index`: the index of the buffer to be returned
    ///
    /// # Return value
    ///
    /// The reference to the desired buffer.
    fn get_buffer(&self, buffer_index: BufferIndex) -> &Buffer {
        // PANIC: BufferIndex guarantees safe access to `buffer_status`
        self.buffers.get(buffer_index.to_usize()).unwrap()
    }

    /// Check if a buffer is available.
    ///
    /// # Parameters
    ///
    /// + `buffer_index`: the index of the buffer to be checked
    ///
    /// # Return value
    ///
    /// + true: the buffer is available
    /// + false: the buffer is not available
    fn is_buffer_available(&self, buffer_index: BufferIndex) -> bool {
        self.get_buffer(buffer_index).is_available()
    }

    /// Get the current buffer index.
    ///
    /// The current buffer index represents the head of the list.
    ///
    /// # Return value
    ///
    /// Current [BufferIndex]
    fn get_current_buffer_index(&self) -> BufferIndex {
        self.current_buffer_index.get()
    }

    /// Advances the current buffer index.
    ///
    /// The current buffer index advances to the next element of the list. If the current buffer
    /// index points to the last element of the list, this method makes it point to the first
    /// element, creating a circular list.
    fn advance_current_buffer_index(&self) {
        let current_buffer_index = self.get_current_buffer_index();
        self.current_buffer_index
            .set(current_buffer_index.next_wrapping());
    }

    /// Finds the next available buffer.
    ///
    /// This method iterates through all buffers and returns the first one available.
    ///
    /// # Return value
    ///
    /// The index of the buffer that is available.
    fn find_available_buffer(&self) -> BufferIndex {
        while !self.is_buffer_available(self.get_current_buffer_index()) {
            self.advance_current_buffer_index();
        }

        self.get_current_buffer_index()
    }

    /// Occupies a buffer.
    ///
    /// Marks a buffer as occupied, i.e. not available.
    ///
    /// # Parameters
    ///
    /// + `buffer_index`: the index of the buffer to be marked as occupied.
    fn occupy_buffer(&self, buffer_index: BufferIndex) {
        self.get_buffer(buffer_index).occupy()
    }

    /// Frees a buffer.
    ///
    /// Marks a buffer as free, i.e. not occupied.
    ///
    /// # Parameters
    ///
    /// + `buffer_index`: the index of the buffer to be marked as free.
    pub(super) fn free_buffer(&self, buffer_index: BufferIndex) {
        self.get_buffer(buffer_index).free()
    }

    /// Finds the next available buffer, occupies it, and then returns an index pointing to it.
    ///
    /// # Return value
    ///
    /// The index of the buffer that has just been occupied.
    pub(super) fn next_and_occupy(&self) -> BufferIndex {
        let available_buffer_index = self.find_available_buffer();
        self.occupy_buffer(available_buffer_index);
        available_buffer_index
    }
}
