// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use super::fifo_level::FifoLevel;
use super::flash_address::FlashAddress;
use super::page::{FlashCtrlPage, RawFlashCtrlPage, EARLGREY_PAGE_SIZE};

use core::num::NonZeroUsize;

/// The size of a word that can be read/write at a time.
const WORD_SIZE: NonZeroUsize =
    // SAFETY: sizeof(usize) = 4 != 0
    unsafe { NonZeroUsize::new_unchecked(core::mem::size_of::<usize>()) };
/// Number of flash words per chunk
pub(super) const WORDS_PER_CHUNK: NonZeroUsize =
    // SAFETY: 16 != 0
    unsafe { NonZeroUsize::new_unchecked(FifoLevel::Level16.inner() as usize) };
/// The size of a chunk in bytes
pub(super) const CHUNK_SIZE: NonZeroUsize = match WORDS_PER_CHUNK.checked_mul(WORD_SIZE) {
    Some(chunk_size) => chunk_size,
    // WORDS_PER_CHUNK * WORD_SIZE = 16 * 4 = 64 ==> multiplication does not overflow.
    None => unreachable!(),
};
/// Number of chunks per flash page
pub(super) const CHUNKS_PER_PAGE: NonZeroUsize =
    // SAFETY: 2048 / 64 = 32 != 0
    unsafe { NonZeroUsize::new_unchecked(EARLGREY_PAGE_SIZE.get() / CHUNK_SIZE.get()) };

/// A chunk of data that can be read/written by a single flash operation
#[repr(transparent)]
pub(super) struct Chunk([usize; WORDS_PER_CHUNK.get()]);

// When running clippy, the architecture is set to the host's architecture and as a consequence,
// align_of::<Chunk>() = align_of::<usize>() = 8, which differs from
// align_of::<RawFlashCtrlPage>() = 4.
#[cfg(target_arch = "riscv32")]
const _CHECK_ALIGNMENT: () = assert!(
    core::mem::align_of::<RawFlashCtrlPage>() == core::mem::align_of::<Chunk>(),
    "RawFlashCtrlPage and Chunk must have the same alignment requirements"
);

// Here, Clippy does not complain, because:
//
// 1. CHUNKS_PER_PAGE = EARLGREY_PAGE_SIZE / CHUNK_SIZE = EARLGREY_PAGE_SIZE / (WORDS_PER_CHUNK *
//    WORD_SIZE)
// 2. size_of::<Chunk>() = size_of::<usize>() * WORDS_PER_CHUNK
//
// Using (1) and (2) ==> CHUNKS_PER_PAGE * size_of::<Chunk>() = EARLGREY_PAGE_SIZE
//
// This compile-time assert is used in case EARLGREY_PAGE_SIZE, CHUNKS_PER_PAGE, or Chunk are
// modified.
const _CHECK_SIZE: () = assert!(
    EARLGREY_PAGE_SIZE.get() == CHUNKS_PER_PAGE.get() * core::mem::size_of::<Chunk>(),
    "CHUNKS_PER_PAGE * size_of::<Chunk>() must be equal to EARLGREY_PAGE_SIZE"
);

impl Chunk {
    /// Return the `index`th word in this chunk
    ///
    /// # Return value
    ///
    /// An immutable reference to the `index`th word
    #[allow(unused)]
    fn get(&self, index: usize) -> Option<&usize> {
        self.0.get(index)
    }

    /// Cast the chunk to an immutable iterator over its content
    ///
    /// # Return value
    ///
    /// An immutable iterator over `self` content
    fn as_iter(&self) -> core::slice::Iter<usize> {
        self.0.iter()
    }

    /// Cast the chunk to an mutable iterator over its content
    ///
    /// # Return value
    ///
    /// An mutable iterator over `self` content
    fn as_mut_iter(&mut self) -> core::slice::IterMut<usize> {
        self.0.iter_mut()
    }
}

/// An iterator over the words of an immutable chunk
pub(super) struct ImmutableChunkIterator<'a>(core::slice::Iter<'a, usize>);

impl<'a> ImmutableChunkIterator<'a> {
    /// [ImmutableChunkIterator] constructor
    ///
    /// # Parameters:
    ///
    /// + `chunk`: the chunk that must be iterated over
    ///
    /// # Return value:
    ///
    /// A new instance of [ImmutableChunkIterator] that starts from the first word of the chunk
    pub(super) fn new(chunk: &'a Chunk) -> Self {
        Self(chunk.as_iter())
    }
}

impl<'a> Iterator for ImmutableChunkIterator<'a> {
    type Item = &'a usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// An iterator over the words of a mutable chunk
pub(super) struct MutableChunkIterator<'a>(core::slice::IterMut<'a, usize>);

impl<'a> MutableChunkIterator<'a> {
    /// [MutableChunkIterator] constructor
    ///
    /// # Parameters:
    ///
    /// + `chunk`: the chunk that must be iterated over
    ///
    /// # Return value:
    ///
    /// A new instance of [MutableChunkIterator] that starts from the first word of the chunk
    pub(super) fn new(chunk: &'a mut Chunk) -> Self {
        Self(chunk.as_mut_iter())
    }
}

impl<'a> Iterator for MutableChunkIterator<'a> {
    type Item = &'a mut usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// The type returned by the immutable page chunk iterator
pub(super) struct ImmutablePageChunkIteratorItem<'a> {
    chunk: &'a Chunk,
    chunk_flash_address: FlashAddress,
}

impl<'a> ImmutablePageChunkIteratorItem<'a> {
    /// [ImmutablePageChunkIteratorItem] constructor
    ///
    /// # Parameters
    ///
    /// + `chunk`: the underlying [Chunk]
    /// + `chunk_flash_address`: the starting [FlashAddress] of `chunk`
    fn new(chunk: &'a Chunk, chunk_flash_address: FlashAddress) -> Self {
        Self {
            chunk,
            chunk_flash_address,
        }
    }

    /// Return the underlying [Chunk] and [FlashAddress]
    ///
    /// # Return value
    ///
    /// + (chunk, flash_address): the underlying [Chunk], [FlashAddress] respectively
    pub(super) fn inner(self) -> (&'a Chunk, FlashAddress) {
        (self.chunk, self.chunk_flash_address)
    }
}

/// Whether a page chunk iterator is empty or not
#[derive(PartialEq, Eq)]
pub(super) enum PageChunkIteratorEmpty {
    /// Empty
    Empty,
    /// Not empty
    NotEmpty,
}

/// A chunk iterator over a flash page
pub(super) struct PageChunkIterator<'a> {
    current_chunk_index: usize,
    chunk_list: &'a mut [Chunk; CHUNKS_PER_PAGE.get()],
    current_chunk_flash_address: FlashAddress,
}

impl<'a> PageChunkIterator<'a> {
    /// [PageChunkIterator] constructor
    ///
    /// # Parameters
    ///
    /// + `page`: the page that must be iterated over
    ///
    /// # Return value
    ///
    /// A new instance of `FlashCtrlPage`.
    pub(super) fn new(page: FlashCtrlPage<'a>) -> Self {
        let page_starting_flash_address = page.get_starting_flash_address();
        let raw_array = page.to_raw_page().as_mut();
        Self {
            current_chunk_index: 0,
            // SAFETY:
            //
            // + it is safe to view a page as a chunk array, since a chunk is a contiguous
            // memory area of 16 machine words
            // + RawFlashCtrlPage is marked as repr(4) == size_of::<usize>() == align_of::<Chunk>()
            chunk_list: unsafe {
                core::mem::transmute::<
                    &'a mut [u8; EARLGREY_PAGE_SIZE.get()],
                    &'a mut [Chunk; CHUNKS_PER_PAGE.get()],
                >(raw_array)
            },
            current_chunk_flash_address: page_starting_flash_address,
        }
    }

    /// Return whether the iterator is empty
    ///
    /// # Return value
    ///
    /// [PageChunkIteratorEmpty] that indicates if the iterator is empty
    pub(super) fn empty(&self) -> PageChunkIteratorEmpty {
        match self.current_chunk_index == CHUNKS_PER_PAGE.get() {
            false => PageChunkIteratorEmpty::NotEmpty,
            true => PageChunkIteratorEmpty::Empty,
        }
    }

    /// Return the next immutable chunk
    ///
    /// # Return value
    ///
    /// + Some(immutable_page_chunk_iterator_item): the next immutable chunk, alongside with its
    /// starting flash address
    /// + None: the iterator is empty
    pub(super) fn next_immutable(&mut self) -> Option<ImmutablePageChunkIteratorItem<'a>> {
        let chunk = self.chunk_list.get(self.current_chunk_index)?;

        // SAFETY: The compiler complains that the returned lifetime of chunk is the lifetime of
        // self instead of 'a. However, since the extracted Chunk is from a chunk list having the
        // lifetime 'a, it is safe to transmute the given lifetime to 'a.
        let chunk = unsafe { core::mem::transmute::<&Chunk, &'a Chunk>(chunk) };

        self.current_chunk_index += 1;
        let chunk_flash_address = self.current_chunk_flash_address;

        // SAFETY:
        //
        // + the addition produces a valid flash address
        unsafe {
            self.current_chunk_flash_address = self
                .current_chunk_flash_address
                .add_unchecked(CHUNK_SIZE.get());
        }

        Some(ImmutablePageChunkIteratorItem::new(
            chunk,
            chunk_flash_address,
        ))
    }

    /// Return the next mutable chunk
    ///
    /// # Return value
    ///
    /// + Some(mutable_page_chunk_iterator_item): the next mutable chunk
    /// + None: the iterator is empty
    pub(super) fn next_mutable(&mut self) -> Option<&'a mut Chunk> {
        let chunk = self.chunk_list.get_mut(self.current_chunk_index)?;

        // SAFETY: The compiler complains that the returned lifetime of chunk is the lifetime of
        // self instead of 'a. However, since the extracted Chunk is from a chunk list having the
        // lifetime 'a, it is safe to transmute the given lifetime to 'a.
        let chunk = unsafe { core::mem::transmute::<&mut Chunk, &'a mut Chunk>(chunk) };

        self.current_chunk_index += 1;

        // SAFETY:
        //
        // + the addition produces a valid flash address
        unsafe {
            self.current_chunk_flash_address = self
                .current_chunk_flash_address
                .add_unchecked(CHUNK_SIZE.get());
        }

        Some(chunk)
    }

    /// Get the starting flash address corresponding to the chunk that would be returned by the
    /// iterator
    pub(super) fn get_current_chunk_flash_addres(&self) -> FlashAddress {
        self.current_chunk_flash_address
    }

    /// Convert the iterator back to a flash page
    ///
    /// # Return value
    ///
    /// The corresponding [RawFlashCtrlPage] for this iterator.
    #[allow(clippy::wrong_self_convention)]
    pub(super) fn to_raw_page(self) -> &'a mut RawFlashCtrlPage {
        // SAFETY: Since RawFlashCtrlPage is marked as repr(C), transmuting between
        // [Chunk; CHUNKS_PER_PAGE] and RawFlashCtrlPage is safe
        unsafe { core::mem::transmute(self.chunk_list) }
    }
}

#[cfg(feature = "test_flash_ctrl")]
pub(in super::super) mod tests {
    use super::super::page::RawFlashCtrlPage;
    use super::super::page_index::DataPageIndex;
    use super::super::page_position::DataPagePosition;
    use super::super::tests::{print_test_footer, print_test_header};
    use super::*;

    impl Chunk {
        fn new(chunk: [usize; WORDS_PER_CHUNK.get()]) -> Self {
            Self(chunk)
        }
    }

    #[cfg_attr(test, test)]
    fn test_immutable_chunk_iterator() {
        print_test_header("ImmutableChunkIterator");

        let array = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let chunk = Chunk::new(array);
        let immutable_chunk_iterator = ImmutableChunkIterator::new(&chunk);

        for (number, word) in immutable_chunk_iterator.enumerate() {
            assert_eq!(number, *word);
        }

        print_test_footer("ImmutableChunkIterator");
    }

    #[cfg_attr(test, test)]
    fn test_mutable_chunk_iterator() {
        print_test_header("MutableChunkIterator");

        let array = [0; WORDS_PER_CHUNK.get()];
        let mut chunk = Chunk::new(array);

        let mutable_chunk_iterator = MutableChunkIterator::new(&mut chunk);

        for (number, word) in mutable_chunk_iterator.enumerate() {
            *word = number;
        }

        for index in 0..WORDS_PER_CHUNK.get() {
            // Panic: index < WORDS_PER_CHUNK, so the call to get() may never panic
            let word = chunk.get(index).unwrap();

            assert_eq!(*word, index);
        }

        print_test_footer("MutableChunkIterator");
    }

    #[cfg_attr(test, test)]
    fn test_page_chunk_iterator() {
        print_test_header("PageChunkIterator");

        let mut raw_page: RawFlashCtrlPage = RawFlashCtrlPage::default();

        let raw_page_number = 0;
        // SAFETY: 0 is a valid data page index
        let data_page_position = DataPagePosition::Bank0(DataPageIndex::new(0));
        let data_page = FlashCtrlPage::new_data_page(data_page_position, &mut raw_page);

        let mut page_chunk_iterator = PageChunkIterator::new(data_page);

        while let Some(chunk) = page_chunk_iterator.next_mutable() {
            let mutable_chunk_iterator = MutableChunkIterator::new(chunk);
            for (number, word) in mutable_chunk_iterator.enumerate() {
                *word = number;
            }
        }

        let mut chunk_index = 0;
        while let Some(immutable_page_chunk_iterator_item) = page_chunk_iterator.next_immutable() {
            // SAFETY:
            //
            // + The previous computation value is valid flash address because:
            //     + raw_page_number * EARLGREY_PAGE_SIZE = 0 * 2048 = 0
            //     + CHUNK_SIZE * chunk_index = [64 * 0; 64 * 31] = [0; 1984]
            //     + the addition of the two values is in the range [0; 1984] which is a valid
            //     flash address
            let expected_flash_address = unsafe {
                FlashAddress::new_unchecked(
                    EARLGREY_PAGE_SIZE.get() * raw_page_number + CHUNK_SIZE.get() * chunk_index,
                )
            };
            assert_eq!(
                immutable_page_chunk_iterator_item.chunk_flash_address, expected_flash_address,
                "Wrong flash address returned by next_immutable()"
            );

            let immutable_chunk_iterator =
                ImmutableChunkIterator::new(immutable_page_chunk_iterator_item.chunk);

            for (number, word) in immutable_chunk_iterator.enumerate() {
                assert_eq!(number, *word);
            }

            chunk_index += 1;
        }

        print_test_footer("PageChunkIterator");
    }

    pub(in super::super) fn run_all() {
        test_immutable_chunk_iterator();
        test_mutable_chunk_iterator();
        test_page_chunk_iterator();
    }
}
