// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use kernel::hil::flash::{
    Error as FlashError, InfoClient as InfoClientTrait, InfoFlash as InfoFlashTrait,
};
use kernel::hil::opentitan_attestation::{
    BootCert, Certificate, CertificateReadError, CertificateReader, CertificateReaderClient,
};
use kernel::utilities::cells::OptionalCell;
use kernel::utilities::cells::TakeCell;
use kernel::{ErrorCode, ProcessId};

/// Location in flash info partition of the UDS certificate (type 0,
/// bank 1, page 6).
const UDS_PAGE: (usize, usize, usize) = (0, 1, 6);
/// Location in flash info partition of the CDI_0 certificate (type 0,
/// bank 1, page 8).
const CDI0_PAGE: (usize, usize, usize) = (0, 1, 8);
/// Location in flash info partition of the CDI_1 certificate (type 0,
/// bank 1, page 9).
const CDI1_PAGE: (usize, usize, usize) = (0, 1, 9);

/// Size in bytes of the in-memory storage for application DICE
/// certificates (6 KiB).
const MEM_CACHE_SIZE: usize = 0x1800;

/// Maximum number of apps the in-memory attestation certificate
/// buffer supports.
const MAX_APPS: usize = 8;

/// Earlgrey-specific driver for attestation
pub struct Attestation<'a, Flash: InfoFlashTrait>
where
    <Flash as InfoFlashTrait>::Page: 'static,
{
    info_flash: &'a Flash,
    flash_buf: TakeCell<'static, Flash::Page>,
    app_certs: AppCerts<MAX_APPS>,
    app_cert_memory_cache: [u8; MEM_CACHE_SIZE],
    cert_off_len: OptionalCell<(usize, Option<usize>)>,
    client: OptionalCell<&'a dyn CertificateReaderClient>,
    owning_process: OptionalCell<ProcessId>,
}

impl<'a, Flash: InfoFlashTrait> Attestation<'a, Flash> {
    /// Create a new attestation driver
    pub fn new(
        info_flash: &'a Flash,
        flash_buf: &'static mut Flash::Page,
    ) -> Attestation<'a, Flash> {
        Attestation {
            info_flash,
            flash_buf: TakeCell::new(flash_buf),
            app_certs: [AppCertsEntry::default(); MAX_APPS],
            app_cert_memory_cache: [0u8; MEM_CACHE_SIZE],
            cert_off_len: OptionalCell::empty(),
            client: OptionalCell::empty(),
            owning_process: OptionalCell::empty(),
        }
    }

    /// Read a specific page from the flash info partition.
    fn read_flash_info_page(
        &self,
        partition_type: usize,
        bank: usize,
        page: usize,
    ) -> Result<(), ErrorCode> {
        let partition_type = match partition_type.try_into() {
            Err(()) => return Err(ErrorCode::FAIL),
            Ok(info_partition_type) => info_partition_type,
        };
        let bank = match bank.try_into() {
            Err(()) => return Err(ErrorCode::INVAL),
            Ok(bank) => bank,
        };
        let buffer = match self.flash_buf.take() {
            None => return Err(ErrorCode::BUSY),
            Some(buffer) => buffer,
        };
        self.info_flash
            .read_info_page(partition_type, bank, page, buffer)
            .map_err(|(err, buf)| {
                self.flash_buf.put(Some(buf));
                err
            })
    }
}

impl<'a, Flash: InfoFlashTrait> CertificateReader<'a> for Attestation<'a, Flash> {
    /// Read a certificate stored in hardware. This implementation
    /// uses the flash controller.
    ///
    /// # Returns
    ///
    /// + `Ok(Some(..))` if the certificate could be read synchronously.
    /// + `Ok(None())` if the certificate is being read asynchronously.
    /// + `Err(..)` if an error occurred.
    fn read_certificate(
        &self,
        calling_process: ProcessId,
        certificate: Certificate,
    ) -> Result<Option<&[u8]>, ErrorCode> {
        if let Some(_) = self.owning_process.get() {
            return Err(ErrorCode::BUSY);
        }
        let (page_type, bank, page, offset, length) = match certificate {
            Certificate::Boot(boot_cert) => {
                // The location of boot-certificates is top- and (on
                // discrete tops) ROM_EXT-specific.
                let (page_type, bank, page) = match boot_cert {
                    BootCert::CreatorIdentity => UDS_PAGE,
                    BootCert::OwnerIntermediate => CDI0_PAGE,
                    BootCert::OwnerIdentity => CDI1_PAGE,
                };
                (page_type, bank, page, 0, None)
            }
            Certificate::Application(process_id) => {
                // For app certificates, we need to look up the
                // location of the certificate for the given process
                // ID in the index, and then find the certificate.
                let entry = match self
                    .app_certs
                    .iter()
                    .find(|ent| ent.process_id == process_id)
                {
                    // Found the index entry
                    Some(entry) => entry,
                    // No index entry for the given process ID
                    None => return Err(ErrorCode::INVAL),
                };
                match entry.location {
                    AppCertLocation::Flash {
                        page_type,
                        bank,
                        page,
                        offset_in_page,
                    } =>
                    // Page is stored in flash.
                    {
                        (page_type, bank, page, offset_in_page, Some(entry.length))
                    }
                    AppCertLocation::Memory { offset_in_buf } =>
                    // Page is stored in memory
                    // cache. Short-circuit flash read and return
                    // certificate slice.
                    {
                        return Ok(Some(
                            &self.app_cert_memory_cache
                                [offset_in_buf..offset_in_buf + entry.length],
                        ))
                    }
                }
            }
        };
        self.owning_process.set(calling_process);
        // Read the requested flash page.
        self.cert_off_len.set((offset, length));
        self.read_flash_info_page(page_type, bank, page)
            .map(|()| None)
    }

    /// Set the client to handle events that a certificate was read.
    fn set_client(&self, client: &'a dyn CertificateReaderClient) {
        self.client.set(client);
    }
}

impl<'a, Flash: InfoFlashTrait> InfoClientTrait<Flash> for Attestation<'a, Flash> {
    fn info_read_complete(
        &self,
        read_buffer: &'static mut Flash::Page,
        status: Result<(), FlashError>,
    ) {
        self.owning_process.map(|owner_id| {
            self.client.map(|client| {
                client.certificate_available(
                    owner_id,
                    status
                        .map(|()| &*read_buffer.as_mut())
                        .map_err(|e| CertificateReadError::Flash(e)),
                );
            });
        });
        self.owning_process.clear();
        self.flash_buf.put(Some(read_buffer));
    }

    fn info_write_complete(
        &self,
        _write_buffer: &'static mut Flash::Page,
        _result: Result<(), FlashError>,
    ) {
        // Should never happen
    }

    fn info_erase_complete(&self, _result: Result<(), FlashError>) {
        // Should never happen
    }
}

type AppCerts<const N: usize> = [AppCertsEntry; N];

#[derive(Clone, Copy, Default)]
struct AppCertsEntry {
    process_id: usize,
    location: AppCertLocation,
    length: usize,
}

#[derive(Clone, Copy)]
// TODO: remove this attribute when the kernel boot flow is added and
// the certificate index is generated alongside the certificates at
// boot-time.
#[allow(dead_code)]
enum AppCertLocation {
    Flash {
        page_type: usize,
        bank: usize,
        page: usize,
        offset_in_page: usize,
    },
    Memory {
        offset_in_buf: usize,
    },
}

impl Default for AppCertLocation {
    fn default() -> AppCertLocation {
        AppCertLocation::Memory { offset_in_buf: 0 }
    }
}
