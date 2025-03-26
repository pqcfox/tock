// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Virtualize writing flash.
//!
//! `MuxFlash` provides shared access to a flash interface from multiple clients
//! in the kernel. For instance, a board may wish to expose the internal MCU
//! flash for multiple uses, like allowing userland apps to write their own
//! flash space, and to provide a "scratch space" as the end of flash for all
//! apps to use. Each of these requires a capsule to support the operation, and
//! must use a `FlashUser` instance to contain the per-user state for the
//! virtualization.
//!
//! Usage
//! -----
//!
//! ```rust,ignore
//! # use kernel::{hil, static_init};
//!
//! // Create the mux.
//! let mux_flash = static_init!(
//!     capsules_core::virtual_flash::MuxFlash<'static, sam4l::flashcalw::FLASHCALW>,
//!     capsules_core::virtual_flash::MuxFlash::new(&sam4l::flashcalw::FLASH_CONTROLLER));
//! hil::flash::HasClient::set_client(&sam4l::flashcalw::FLASH_CONTROLLER, mux_flash);
//!
//! // Everything that then uses the virtualized flash must use one of these.
//! let virtual_flash = static_init!(
//!     capsules_core::virtual_flash::FlashUser<'static, sam4l::flashcalw::FLASHCALW>,
//!     capsules_core::virtual_flash::FlashUser::new(mux_flash));
//! ```

use core::cell::Cell;

use kernel::collections::list::{List, ListLink, ListNode};
use kernel::hil;
use kernel::hil::flash::Error as FlashError;
use kernel::utilities::cells::{OptionalCell, TakeCell};
use kernel::ErrorCode;

/// Handle keeping a list of active users of flash hardware and serialize their
/// requests.
///
/// After each completed request the list is checked to see if there
/// is another flash user with an outstanding read, write, or erase
/// request.
pub struct MuxFlash<'a, F: hil::flash::Flash + 'static> {
    flash: &'a F,
    users: List<'a, FlashUser<'a, F>>,
    inflight: OptionalCell<&'a FlashUser<'a, F>>,
}

impl<F: hil::flash::Flash> hil::flash::Client<F> for MuxFlash<'_, F> {
    fn read_complete(
        &self,
        pagebuffer: &'static mut F::Page,
        result: Result<(), hil::flash::Error>,
    ) {
        self.inflight.take().map(move |user| {
            user.read_complete(pagebuffer, result);
        });
        self.do_next_op();
    }

    fn write_complete(
        &self,
        pagebuffer: &'static mut F::Page,
        result: Result<(), hil::flash::Error>,
    ) {
        self.inflight.take().map(move |user| {
            user.write_complete(pagebuffer, result);
        });
        self.do_next_op();
    }

    fn erase_complete(&self, result: Result<(), hil::flash::Error>) {
        self.inflight.take().map(move |user| {
            user.erase_complete(result);
        });
        self.do_next_op();
    }
}

impl<'a, F: hil::flash::Flash> MuxFlash<'a, F> {
    pub const fn new(flash: &'a F) -> MuxFlash<'a, F> {
        MuxFlash {
            flash,
            users: List::new(),
            inflight: OptionalCell::empty(),
        }
    }

    /// Scan the list of users and find the first user that has a pending
    /// request, then issue that request to the flash hardware.
    fn do_next_op(&self) {
        if self.inflight.is_none() {
            let mnode = self
                .users
                .iter()
                .find(|node| node.operation.get() != Op::Idle);
            mnode.map(|node| {
                node.buffer.take().map_or_else(
                    || {
                        // Don't need a buffer for erase.
                        match node.operation.get() {
                            Op::Erase(page_number) => {
                                let _ = self.flash.erase_page(page_number);
                            }
                            _ => {}
                        };
                    },
                    |buf| {
                        match node.operation.get() {
                            Op::Write(page_number) => {
                                if let Err((_, buf)) = self.flash.write_page(page_number, buf) {
                                    node.buffer.replace(buf);
                                }
                            }
                            Op::Read(page_number) => {
                                if let Err((_, buf)) = self.flash.read_page(page_number, buf) {
                                    node.buffer.replace(buf);
                                }
                            }
                            Op::Erase(page_number) => {
                                let _ = self.flash.erase_page(page_number);
                            }
                            Op::Idle => {} // Can't get here...
                        }
                    },
                );
                node.operation.set(Op::Idle);
                self.inflight.set(node);
            });
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Op {
    Idle,
    Write(usize),
    Read(usize),
    Erase(usize),
}

/// Keeps state for each flash user.
///
/// All uses of the virtualized flash interface need to create one of
/// these to be a user of the flash. The `new()` function handles most
/// of the work, a user only has to pass in a reference to the
/// MuxFlash object.
pub struct FlashUser<'a, F: hil::flash::Flash + 'static> {
    mux: &'a MuxFlash<'a, F>,
    buffer: TakeCell<'static, F::Page>,
    operation: Cell<Op>,
    next: ListLink<'a, FlashUser<'a, F>>,
    client: OptionalCell<&'a dyn hil::flash::Client<FlashUser<'a, F>>>,
}

impl<'a, F: hil::flash::Flash> FlashUser<'a, F> {
    pub fn new(mux: &'a MuxFlash<'a, F>) -> FlashUser<'a, F> {
        FlashUser {
            mux,
            buffer: TakeCell::empty(),
            operation: Cell::new(Op::Idle),
            next: ListLink::empty(),
            client: OptionalCell::empty(),
        }
    }
}

impl<'a, F: hil::flash::Flash, C: hil::flash::Client<Self>> hil::flash::HasClient<'a, C>
    for FlashUser<'a, F>
{
    fn set_client(&'a self, client: &'a C) {
        self.mux.users.push_head(self);
        self.client.set(client);
    }
}

impl<F: hil::flash::Flash> hil::flash::Client<F> for FlashUser<'_, F> {
    fn read_complete(
        &self,
        pagebuffer: &'static mut F::Page,
        result: Result<(), hil::flash::Error>,
    ) {
        self.client.map(move |client| {
            client.read_complete(pagebuffer, result);
        });
    }

    fn write_complete(
        &self,
        pagebuffer: &'static mut F::Page,
        result: Result<(), hil::flash::Error>,
    ) {
        self.client.map(move |client| {
            client.write_complete(pagebuffer, result);
        });
    }

    fn erase_complete(&self, result: Result<(), hil::flash::Error>) {
        self.client.map(move |client| {
            client.erase_complete(result);
        });
    }
}

impl<'a, F: hil::flash::Flash> ListNode<'a, FlashUser<'a, F>> for FlashUser<'a, F> {
    fn next(&'a self) -> &'a ListLink<'a, FlashUser<'a, F>> {
        &self.next
    }
}

impl<F: hil::flash::Flash> hil::flash::Flash for FlashUser<'_, F> {
    type Page = F::Page;

    fn read_page(
        &self,
        page_number: usize,
        buf: &'static mut Self::Page,
    ) -> Result<(), (ErrorCode, &'static mut Self::Page)> {
        self.buffer.replace(buf);
        self.operation.set(Op::Read(page_number));
        self.mux.do_next_op();
        Ok(())
    }

    fn write_page(
        &self,
        page_number: usize,
        buf: &'static mut Self::Page,
    ) -> Result<(), (ErrorCode, &'static mut Self::Page)> {
        self.buffer.replace(buf);
        self.operation.set(Op::Write(page_number));
        self.mux.do_next_op();
        Ok(())
    }

    fn erase_page(&self, page_number: usize) -> Result<(), ErrorCode> {
        self.operation.set(Op::Erase(page_number));
        self.mux.do_next_op();
        Ok(())
    }
}

// Info flash multiplexer

/// Handle keeping a list of active users of flash hardware and serialize their
/// requests. After each completed request the list is checked to see if there
/// is another flash user with an outstanding read, write, or erase request.
pub struct MuxInfoFlash<'a, F: hil::flash::InfoFlash + 'static>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    info_flash: &'a F,
    users: List<'a, InfoFlashUser<'a, F>>,
    inflight: OptionalCell<&'a InfoFlashUser<'a, F>>,
}

impl<F: hil::flash::InfoFlash> hil::flash::InfoClient<F> for MuxInfoFlash<'_, F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    fn info_read_complete(
        &self,
        pagebuffer: &'static mut F::Page,
        result: Result<(), hil::flash::Error>,
    ) {
        self.inflight.take().map(move |user| {
            user.info_read_complete(pagebuffer, result);
        });
        // If any of the following operations fail-fast, notify the caller immediately that their
        // operation failed.
        while let Err(_) = self.do_next_op() {
            self.inflight.take().map(move |user| {
                user.info_erase_complete(Err(FlashError::FlashError));
            });
        }
    }

    fn info_write_complete(
        &self,
        pagebuffer: &'static mut F::Page,
        result: Result<(), hil::flash::Error>,
    ) {
        self.inflight.take().map(move |user| {
            user.info_write_complete(pagebuffer, result);
        });
        // If any of the following operations fail-fast, notify the caller immediately that their
        // operation failed.
        while let Err(_) = self.do_next_op() {
            self.inflight.take().map(move |user| {
                user.info_erase_complete(Err(FlashError::FlashError));
            });
        }
    }

    fn info_erase_complete(&self, result: Result<(), hil::flash::Error>) {
        self.inflight.take().map(move |user| {
            user.info_erase_complete(result);
        });
        // If any of the following operations fail-fast, notify the caller immediately that their
        // operation failed.
        while let Err(_) = self.do_next_op() {
            self.inflight.take().map(move |user| {
                user.info_erase_complete(Err(FlashError::FlashError));
            });
        }
    }
}

impl<'a, F: hil::flash::InfoFlash> MuxInfoFlash<'a, F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    pub const fn new(flash: &'a F) -> MuxInfoFlash<'a, F> {
        MuxInfoFlash {
            info_flash: flash,
            users: List::new(),
            inflight: OptionalCell::empty(),
        }
    }

    /// Scan the list of users and find the first user that has a pending
    /// request, then issue that request to the flash hardware.
    fn do_next_op(&self) -> Result<(), ErrorCode> {
        if self.inflight.is_none() {
            let mnode = self.users.iter().find(|node| match node.operation.get() {
                InfoOp::Idle => false,
                _ => true,
            });
            mnode.map_or(Ok(()), |node| {
                let result = node.buffer.take().map_or_else(
                    || {
                        // Don't need a buffer for erase.
                        match node.operation.get() {
                            InfoOp::Erase(info_type, bank, page_number) => self
                                .info_flash
                                .erase_info_page(info_type, bank, page_number),
                            _ => Err(ErrorCode::FAIL),
                        }
                    },
                    |buf| {
                        match node.operation.get() {
                            InfoOp::Write(info_type, bank, page_number) => {
                                if let Err((err, buf)) = self.info_flash.write_info_page(
                                    info_type,
                                    bank,
                                    page_number,
                                    buf,
                                ) {
                                    node.buffer.replace(buf);
                                    return Err(err);
                                }
                                Ok(())
                            }
                            InfoOp::Read(info_type, bank, page_number) => {
                                if let Err((err, buf)) = self.info_flash.read_info_page(
                                    info_type,
                                    bank,
                                    page_number,
                                    buf,
                                ) {
                                    node.buffer.replace(buf);
                                    return Err(err);
                                }
                                Ok(())
                            }
                            InfoOp::Erase(info_type, bank, page_number) => self
                                .info_flash
                                .erase_info_page(info_type, bank, page_number),
                            InfoOp::Idle => Err(ErrorCode::FAIL), // Can't get here...
                        }
                    },
                );
                node.operation.set(InfoOp::Idle);
                self.inflight.set(node);
                result
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum InfoOp<P: Copy, B: Copy> {
    Idle,
    Write(P, B, usize),
    Read(P, B, usize),
    Erase(P, B, usize),
}

/// Keep state for each flash user. All uses of the virtualized flash interface
/// need to create one of these to be a user of the flash. The `new()` function
/// handles most of the work, a user only has to pass in a reference to the
/// MuxFlash object.
pub struct InfoFlashUser<'a, F: hil::flash::InfoFlash + 'static>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    mux: &'a MuxInfoFlash<'a, F>,
    buffer: TakeCell<'static, F::Page>,
    operation: Cell<InfoOp<F::InfoType, F::BankType>>,
    next: ListLink<'a, InfoFlashUser<'a, F>>,
    client: OptionalCell<&'a dyn hil::flash::InfoClient<InfoFlashUser<'a, F>>>,
}

impl<'a, F: hil::flash::InfoFlash> InfoFlashUser<'a, F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    pub fn new(mux: &'a MuxInfoFlash<'a, F>) -> InfoFlashUser<'a, F> {
        InfoFlashUser {
            mux,
            buffer: TakeCell::empty(),
            operation: Cell::new(InfoOp::Idle),
            next: ListLink::empty(),
            client: OptionalCell::empty(),
        }
    }
}

impl<'a, F: hil::flash::InfoFlash, C: hil::flash::InfoClient<Self>> hil::flash::HasInfoClient<'a, C>
    for InfoFlashUser<'a, F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    fn set_info_client(&'a self, client: &'a C) {
        self.mux.users.push_head(self);
        self.client.set(client);
    }
}

impl<'a, F: hil::flash::InfoFlash> hil::flash::InfoClient<F> for InfoFlashUser<'a, F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    fn info_read_complete(
        &self,
        pagebuffer: &'static mut F::Page,
        result: Result<(), hil::flash::Error>,
    ) {
        self.client.map(move |client| {
            client.info_read_complete(pagebuffer, result);
        });
    }

    fn info_write_complete(
        &self,
        pagebuffer: &'static mut F::Page,
        result: Result<(), hil::flash::Error>,
    ) {
        self.client.map(move |client| {
            client.info_write_complete(pagebuffer, result);
        });
    }

    fn info_erase_complete(&self, result: Result<(), hil::flash::Error>) {
        self.client.map(move |client| {
            client.info_erase_complete(result);
        });
    }
}

impl<'a, F: hil::flash::InfoFlash> ListNode<'a, InfoFlashUser<'a, F>> for InfoFlashUser<'a, F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    fn next(&'a self) -> &'a ListLink<'a, InfoFlashUser<'a, F>> {
        &self.next
    }
}

impl<F: hil::flash::InfoFlash> hil::flash::InfoFlash for InfoFlashUser<'_, F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    type InfoType = F::InfoType;
    type BankType = F::BankType;
    type Page = F::Page;

    fn read_info_page(
        &self,
        info_type: Self::InfoType,
        bank: Self::BankType,
        page_number: usize,
        buf: &'static mut Self::Page,
    ) -> Result<(), (ErrorCode, &'static mut Self::Page)> {
        self.buffer.replace(buf);
        self.operation
            .set(InfoOp::Read(info_type, bank, page_number));
        self.mux.do_next_op().map_err(|err| {
            // PANIC: `self.buffer` cannot be empty because we set it at the beginning of the
            // function.
            (err, self.buffer.take().unwrap())
        })
    }

    fn write_info_page(
        &self,
        info_type: Self::InfoType,
        bank: Self::BankType,
        page_number: usize,
        buf: &'static mut Self::Page,
    ) -> Result<(), (ErrorCode, &'static mut Self::Page)> {
        self.buffer.replace(buf);
        self.operation
            .set(InfoOp::Write(info_type, bank, page_number));
        self.mux.do_next_op().map_err(|err| {
            // PANIC: `self.buffer` cannot be empty because we set it at the beginning of the
            // function.
            (err, self.buffer.take().unwrap())
        })
    }

    fn erase_info_page(
        &self,
        info_type: Self::InfoType,
        bank: Self::BankType,
        page_number: usize,
    ) -> Result<(), ErrorCode> {
        self.operation
            .set(InfoOp::Erase(info_type, bank, page_number));
        self.mux.do_next_op()
    }
}
