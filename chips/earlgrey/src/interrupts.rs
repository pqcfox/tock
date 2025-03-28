// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Top-level interrupt handler for Earlgrey.

use crate::alert_handler::AlertClass;
use crate::chip::EarlGreyDefaultPeripherals;
use crate::chip_config::EarlGreyConfig;
use crate::flash_ctrl::FlashCtrlInterrupt;
use crate::pinmux_config::EarlGreyPinmuxConfig;
use crate::registers::top_earlgrey::PlicIrqId;
use crate::sensor_ctrl::SensorCtrlInterrupt;
use kernel::platform::chip::InterruptService;
use lowrisc::adc_ctrl::AdcCtrlInterrupt;
use lowrisc::aon_timer::AonTimerInterrupt;
use lowrisc::csrng::CsrngInterrupt;
use lowrisc::edn::EdnInterrupt;
use lowrisc::entropy_src::EntropySrcInterrupt;
use lowrisc::gpio::GpioInterrupt;
use lowrisc::hmac::HmacInterrupt;
use lowrisc::i2c::I2cInterrupt;
use lowrisc::keymgr::KeymgrInterrupt;
use lowrisc::kmac::KmacInterrupt;
use lowrisc::otbn::OtbnInterrupt;
use lowrisc::otp::OtpCtrlInterrupt;
use lowrisc::pattgen::PattgenInterrupt;
use lowrisc::spi_device::SpiDeviceInterrupt;
use lowrisc::spi_host::SpiHostInterrupt;
#[cfg(not(feature = "qemu"))]
use lowrisc::sysrst_ctrl::SysRstCtrlInterrupt;
use lowrisc::timer::RvTimerInterrupt;
use lowrisc::uart::UartInterrupt;
use lowrisc::usb::UsbInterrupt;

// Macro that:
// - Generates the top-level interrupt handler based on individual handlers
// - Generates a function that selectively enables interrupts depending on
//   whether the associated driver is included in the kernel configuration.
macro_rules! interrupts {
    [
        $({$peripheral:ident, $plic_name:ident, $local_name:expr},)*
    ] => {
        /// Top-level handler for interrupts from Earlgrey peripherals.
        impl<'a, CFG: EarlGreyConfig, PINMUX: EarlGreyPinmuxConfig> InterruptService
            for EarlGreyDefaultPeripherals<'a, CFG, PINMUX>
        where
            CFG: EarlGreyConfig,
            PINMUX: EarlGreyPinmuxConfig,
        {
            unsafe fn service_interrupt(&self, interrupt: u32) -> bool {
                let interrupt = PlicIrqId::try_from(interrupt).expect("Invalid interrupt ID");

                match interrupt {
                    // Special cases:
                    //
                    // No-op
                    PlicIrqId::None => {},
                    // Unreachable; handled by calling function
                    PlicIrqId::PwrmgrAonWakeup => {},
                    // GPIOs are array-indexed and do not follow the pattern below.
                    PlicIrqId::GpioGpio0 => self.handle_gpio_interrupt(0),
                    PlicIrqId::GpioGpio1 => self.handle_gpio_interrupt(1),
                    PlicIrqId::GpioGpio2 => self.handle_gpio_interrupt(2),
                    PlicIrqId::GpioGpio3 => self.handle_gpio_interrupt(3),
                    PlicIrqId::GpioGpio4 => self.handle_gpio_interrupt(4),
                    PlicIrqId::GpioGpio5 => self.handle_gpio_interrupt(5),
                    PlicIrqId::GpioGpio6 => self.handle_gpio_interrupt(6),
                    PlicIrqId::GpioGpio7 => self.handle_gpio_interrupt(7),
                    PlicIrqId::GpioGpio8 => self.handle_gpio_interrupt(8),
                    PlicIrqId::GpioGpio9 => self.handle_gpio_interrupt(9),
                    PlicIrqId::GpioGpio10 => self.handle_gpio_interrupt(10),
                    PlicIrqId::GpioGpio11 => self.handle_gpio_interrupt(11),
                    PlicIrqId::GpioGpio12 => self.handle_gpio_interrupt(12),
                    PlicIrqId::GpioGpio13 => self.handle_gpio_interrupt(13),
                    PlicIrqId::GpioGpio14 => self.handle_gpio_interrupt(14),
                    PlicIrqId::GpioGpio15 => self.handle_gpio_interrupt(15),
                    PlicIrqId::GpioGpio16 => self.handle_gpio_interrupt(16),
                    PlicIrqId::GpioGpio17 => self.handle_gpio_interrupt(17),
                    PlicIrqId::GpioGpio18 => self.handle_gpio_interrupt(18),
                    PlicIrqId::GpioGpio19 => self.handle_gpio_interrupt(19),
                    PlicIrqId::GpioGpio20 => self.handle_gpio_interrupt(20),
                    PlicIrqId::GpioGpio21 => self.handle_gpio_interrupt(21),
                    PlicIrqId::GpioGpio22 => self.handle_gpio_interrupt(22),
                    PlicIrqId::GpioGpio23 => self.handle_gpio_interrupt(23),
                    PlicIrqId::GpioGpio24 => self.handle_gpio_interrupt(24),
                    PlicIrqId::GpioGpio25 => self.handle_gpio_interrupt(25),
                    PlicIrqId::GpioGpio26 => self.handle_gpio_interrupt(26),
                    PlicIrqId::GpioGpio27 => self.handle_gpio_interrupt(27),
                    PlicIrqId::GpioGpio28 => self.handle_gpio_interrupt(28),
                    PlicIrqId::GpioGpio29 => self.handle_gpio_interrupt(29),
                    PlicIrqId::GpioGpio30 => self.handle_gpio_interrupt(30),
                    PlicIrqId::GpioGpio31 => self.handle_gpio_interrupt(31),


                    // TODO: This is temporary until we can remove the "qemu"
                    // feature gate on `chip.sysreset` by optioning out the
                    // unused peripherals.
                    #[cfg(not(feature = "qemu"))]
                    PlicIrqId::SysrstCtrlAonEventDetected => {
                        self.sysreset.handle_interrupt(SysRstCtrlInterrupt::AonEventDetected)
                    },
                    #[cfg(feature = "qemu")]
                    PlicIrqId::SysrstCtrlAonEventDetected => {},

                    $(PlicIrqId::$plic_name => { self.$peripheral.handle_interrupt($local_name); },)*
                }
                true
            }
        }

        impl<'a, CFG: EarlGreyConfig, PINMUX: EarlGreyPinmuxConfig>
            EarlGreyDefaultPeripherals<'a, CFG, PINMUX>
        where
            CFG: EarlGreyConfig,
            PINMUX: EarlGreyPinmuxConfig,
        {
            #[inline]
            fn handle_gpio_interrupt(&self, pin: usize) {
                // The `map` should always produce a driver, because we do not
                // enable interrupts for any drivers that are not part of the
                // configuration (see `enable_plic_interrupts` below).
                self.gpio_port[pin].handle_interrupt(GpioInterrupt::Gpio);
            }
        }
    }
}

interrupts! [
    // Format: { peripheral, PLIC interrupt name, driver interrupt name }
    //
    // UART0 interrupts
    { uart0, Uart0TxEmpty, UartInterrupt::TxEmpty },
    { uart0, Uart0RxParityErr, UartInterrupt::RxParityErr },
    { uart0, Uart0RxTimeout, UartInterrupt::RxTimeout },
    { uart0, Uart0RxBreakErr, UartInterrupt::RxBreakErr },
    { uart0, Uart0RxFrameErr, UartInterrupt::RxFrameErr },
    { uart0, Uart0RxOverflow, UartInterrupt::RxOverflow },
    { uart0, Uart0TxDone, UartInterrupt::TxDone },
    { uart0, Uart0RxWatermark, UartInterrupt::RxWatermark },
    { uart0, Uart0TxWatermark, UartInterrupt::TxWatermark },


    // UART1 interrupts
    { uart1, Uart1TxEmpty, UartInterrupt::TxEmpty },
    { uart1, Uart1RxParityErr, UartInterrupt::RxParityErr },
    { uart1, Uart1RxTimeout, UartInterrupt::RxTimeout },
    { uart1, Uart1RxBreakErr, UartInterrupt::RxBreakErr },
    { uart1, Uart1RxFrameErr, UartInterrupt::RxFrameErr },
    { uart1, Uart1RxOverflow, UartInterrupt::RxOverflow },
    { uart1, Uart1TxDone, UartInterrupt::TxDone },
    { uart1, Uart1RxWatermark, UartInterrupt::RxWatermark },
    { uart1, Uart1TxWatermark, UartInterrupt::TxWatermark },

    // UART2 interrupts
    { uart2, Uart2TxEmpty, UartInterrupt::TxEmpty },
    { uart2, Uart2RxParityErr, UartInterrupt::RxParityErr },
    { uart2, Uart2RxTimeout, UartInterrupt::RxTimeout },
    { uart2, Uart2RxBreakErr, UartInterrupt::RxBreakErr },
    { uart2, Uart2RxFrameErr, UartInterrupt::RxFrameErr },
    { uart2, Uart2RxOverflow, UartInterrupt::RxOverflow },
    { uart2, Uart2TxDone, UartInterrupt::TxDone },
    { uart2, Uart2RxWatermark, UartInterrupt::RxWatermark },
    { uart2, Uart2TxWatermark, UartInterrupt::TxWatermark },

    // UART3 interrupts
    { uart3, Uart3TxEmpty, UartInterrupt::TxEmpty },
    { uart3, Uart3RxParityErr, UartInterrupt::RxParityErr },
    { uart3, Uart3RxTimeout, UartInterrupt::RxTimeout },
    { uart3, Uart3RxBreakErr, UartInterrupt::RxBreakErr },
    { uart3, Uart3RxFrameErr, UartInterrupt::RxFrameErr },
    { uart3, Uart3RxOverflow, UartInterrupt::RxOverflow },
    { uart3, Uart3TxDone, UartInterrupt::TxDone },
    { uart3, Uart3RxWatermark, UartInterrupt::RxWatermark },
    { uart3, Uart3TxWatermark, UartInterrupt::TxWatermark },

    // SPI Device interrupts
    { spi_device, SpiDeviceUploadCmdfifoNotEmpty, SpiDeviceInterrupt::UploadCmdfifoNotEmpty },
    { spi_device, SpiDeviceUploadPayloadNotEmpty, SpiDeviceInterrupt::UploadPayloadNotEmpty },
    { spi_device, SpiDeviceUploadPayloadOverflow, SpiDeviceInterrupt::UploadPayloadOverflow },
    { spi_device, SpiDeviceReadbufWatermark, SpiDeviceInterrupt::ReadbufWatermark },
    { spi_device, SpiDeviceReadbufFlip, SpiDeviceInterrupt::ReadbufFlip },
    { spi_device, SpiDeviceTpmHeaderNotEmpty, SpiDeviceInterrupt::TpmHeaderNotEmpty },
    { spi_device, SpiDeviceTpmRdfifoCmdEnd, SpiDeviceInterrupt::TpmRdfifoCmdEnd },
    { spi_device, SpiDeviceTpmRdfifoDrop, SpiDeviceInterrupt::TpmRdfifoDrop },

    // I2C0 interrupts
    { i2c0, I2c0FmtThreshold, I2cInterrupt::FmtThreshold },
    { i2c0, I2c0RxThreshold, I2cInterrupt::RxThreshold },
    { i2c0, I2c0AcqThreshold, I2cInterrupt::AcqThreshold },
    { i2c0, I2c0RxOverflow, I2cInterrupt::RxOverflow },
    { i2c0, I2c0ControllerHalt, I2cInterrupt::ControllerHalt },
    { i2c0, I2c0SclInterference, I2cInterrupt::SclInterference },
    { i2c0, I2c0SdaInterference, I2cInterrupt::SdaInterference },
    { i2c0, I2c0StretchTimeout, I2cInterrupt::StretchTimeout },
    { i2c0, I2c0SdaUnstable, I2cInterrupt::SdaUnstable },
    { i2c0, I2c0CmdComplete, I2cInterrupt::CmdComplete },
    { i2c0, I2c0TxStretch, I2cInterrupt::TxStretch },
    { i2c0, I2c0TxThreshold, I2cInterrupt::TxThreshold },
    { i2c0, I2c0AcqStretch, I2cInterrupt::AcqStretch },
    { i2c0, I2c0UnexpStop, I2cInterrupt::UnexpStop },
    { i2c0, I2c0HostTimeout, I2cInterrupt::HostTimeout },

    // I2C1 interrupts
    { i2c1, I2c1FmtThreshold, I2cInterrupt::FmtThreshold },
    { i2c1, I2c1RxThreshold, I2cInterrupt::RxThreshold },
    { i2c1, I2c1AcqThreshold, I2cInterrupt::AcqThreshold },
    { i2c1, I2c1RxOverflow, I2cInterrupt::RxOverflow },
    { i2c1, I2c1ControllerHalt, I2cInterrupt::ControllerHalt },
    { i2c1, I2c1SclInterference, I2cInterrupt::SclInterference },
    { i2c1, I2c1SdaInterference, I2cInterrupt::SdaInterference },
    { i2c1, I2c1StretchTimeout, I2cInterrupt::StretchTimeout },
    { i2c1, I2c1SdaUnstable, I2cInterrupt::SdaUnstable },
    { i2c1, I2c1CmdComplete, I2cInterrupt::CmdComplete },
    { i2c1, I2c1TxStretch, I2cInterrupt::TxStretch },
    { i2c1, I2c1TxThreshold, I2cInterrupt::TxThreshold },
    { i2c1, I2c1AcqStretch, I2cInterrupt::AcqStretch },
    { i2c1, I2c1UnexpStop, I2cInterrupt::UnexpStop },
    { i2c1, I2c1HostTimeout, I2cInterrupt::HostTimeout },

    // I2C2 interrupts
    { i2c2, I2c2FmtThreshold, I2cInterrupt::FmtThreshold },
    { i2c2, I2c2RxThreshold, I2cInterrupt::RxThreshold },
    { i2c2, I2c2AcqThreshold, I2cInterrupt::AcqThreshold },
    { i2c2, I2c2RxOverflow, I2cInterrupt::RxOverflow },
    { i2c2, I2c2ControllerHalt, I2cInterrupt::ControllerHalt },
    { i2c2, I2c2SclInterference, I2cInterrupt::SclInterference },
    { i2c2, I2c2SdaInterference, I2cInterrupt::SdaInterference },
    { i2c2, I2c2StretchTimeout, I2cInterrupt::StretchTimeout },
    { i2c2, I2c2SdaUnstable, I2cInterrupt::SdaUnstable },
    { i2c2, I2c2CmdComplete, I2cInterrupt::CmdComplete },
    { i2c2, I2c2TxStretch, I2cInterrupt::TxStretch },
    { i2c2, I2c2TxThreshold, I2cInterrupt::TxThreshold },
    { i2c2, I2c2AcqStretch, I2cInterrupt::AcqStretch },
    { i2c2, I2c2UnexpStop, I2cInterrupt::UnexpStop },
    { i2c2, I2c2HostTimeout, I2cInterrupt::HostTimeout },

    // Pattgen interrupts
    { pattgen, PattgenDoneCh0, PattgenInterrupt::Channel0Done },
    { pattgen, PattgenDoneCh1, PattgenInterrupt::Channel1Done },

    // RvTimer interrupts
    { timer, RvTimerTimerExpiredHart0Timer0, RvTimerInterrupt::ExpiredHart0Timer0 },

    // OTP Controller interrupts
    { otp, OtpCtrlOtpOperationDone, OtpCtrlInterrupt::OtpOperationDone },
    { otp, OtpCtrlOtpError, OtpCtrlInterrupt::OtpError },

    // Alert Handler interrupts
    { alert_handler, AlertHandlerClassa, AlertClass::ClassA },
    { alert_handler, AlertHandlerClassb, AlertClass::ClassB },
    { alert_handler, AlertHandlerClassc, AlertClass::ClassC },
    { alert_handler, AlertHandlerClassd, AlertClass::ClassD },

    // SPI Host0 interrupts
    { spi_host0, SpiHost0Error, SpiHostInterrupt::Error },
    { spi_host0, SpiHost0SpiEvent, SpiHostInterrupt::Event },

    // SPI Host1 interrupts
    { spi_host1, SpiHost1Error, SpiHostInterrupt::Error },
    { spi_host1, SpiHost1SpiEvent, SpiHostInterrupt::Event },

    // USBDEV interrupts
    { usb, UsbdevPktReceived, UsbInterrupt::PacketReceived },
    { usb, UsbdevPktSent, UsbInterrupt::PacketSent },
    { usb, UsbdevDisconnected, UsbInterrupt::Disconnected },
    { usb, UsbdevHostLost, UsbInterrupt::HostLost },
    { usb, UsbdevLinkReset, UsbInterrupt::LinkReset },
    { usb, UsbdevLinkSuspend, UsbInterrupt::LinkSuspended },
    { usb, UsbdevLinkResume, UsbInterrupt::LinkResume },
    { usb, UsbdevAvOutEmpty, UsbInterrupt::AvOutEmpty },
    { usb, UsbdevRxFull, UsbInterrupt::RxFull },
    { usb, UsbdevAvOverflow, UsbInterrupt::AvOverflow },
    { usb, UsbdevLinkInErr, UsbInterrupt::LinkInErr },
    { usb, UsbdevRxCrcErr, UsbInterrupt::RxCrcErr },
    { usb, UsbdevRxPidErr, UsbInterrupt::RxPidErr },
    { usb, UsbdevRxBitstuffErr, UsbInterrupt::RxBitstuffErr },
    { usb, UsbdevFrame, UsbInterrupt::Frame },
    { usb, UsbdevPowered, UsbInterrupt::Powered },
    { usb, UsbdevLinkOutErr, UsbInterrupt::LinkOutErr },
    { usb, UsbdevAvSetupEmpty, UsbInterrupt::AvSetupEmpty },


    // TODO: This is temporarily commented out until we can remove the "qemu"
    // feature gate on `chip.sysreset` by optioning out the unused peripherals.
    //
    // System Reset Controller interrupts
    //{ sysreset, SysrstCtrlAonEventDetected, SysRstCtrlInterrupt::AonEventDetected },

    // ADC Controller interrupts
    { adc_ctrl, AdcCtrlAonMatchPending, AdcCtrlInterrupt::MatchPending },

    // AON Timer interrupts
    { watchdog, AonTimerAonWkupTimerExpired, AonTimerInterrupt::AonWkupTimerExpired },
    { watchdog, AonTimerAonWdogTimerBark, AonTimerInterrupt::AonWdogTimerBark },

    // Sensor Control interrupts
    { sensor_ctrl, SensorCtrlAonIoStatusChange, SensorCtrlInterrupt::IoStatusChange },
    { sensor_ctrl, SensorCtrlAonInitStatusChange, SensorCtrlInterrupt::InitStatusChange },

    // Flash Controller interrupts
    { flash_ctrl, FlashCtrlProgEmpty, FlashCtrlInterrupt::ProgEmpty },
    { flash_ctrl, FlashCtrlProgLvl, FlashCtrlInterrupt::ProgLvl },
    { flash_ctrl, FlashCtrlRdFull, FlashCtrlInterrupt::RdFull },
    { flash_ctrl, FlashCtrlRdLvl, FlashCtrlInterrupt::RdLvl },
    { flash_ctrl, FlashCtrlOpDone, FlashCtrlInterrupt::OpDone },
    { flash_ctrl, FlashCtrlCorrErr, FlashCtrlInterrupt::CorrErr },

    // HMAC interrupts
    { hmac, HmacHmacDone, HmacInterrupt::HmacDone },
    { hmac, HmacFifoEmpty, HmacInterrupt::FifoEmpty },
    { hmac, HmacHmacErr, HmacInterrupt::HmacErr },

    // KMAC interrupts
    { kmac, KmacKmacDone, KmacInterrupt::KmacDone },
    { kmac, KmacFifoEmpty, KmacInterrupt::FifoEmpty },
    { kmac, KmacKmacErr, KmacInterrupt::KmacErr },

    // OTBN interrupts
    { otbn, OtbnDone, OtbnInterrupt::Done },

    // Key Manager interrupts
    { keymgr, KeymgrOpDone, KeymgrInterrupt::OpDone },

    // CSRNG interrupts
    { csrng, CsrngCsCmdReqDone, CsrngInterrupt::CsCmdReqDone },
    { csrng, CsrngCsEntropyReq, CsrngInterrupt::CsEntropyReq },
    { csrng, CsrngCsHwInstExc, CsrngInterrupt::CsHwInstExc },
    { csrng, CsrngCsFatalErr, CsrngInterrupt::CsFatalErr },

    // Entropy Source interrupts
    { entropy_src, EntropySrcEsEntropyValid, EntropySrcInterrupt::EsEntropyValid },
    { entropy_src, EntropySrcEsHealthTestFailed, EntropySrcInterrupt::EsHealthTestFailed },
    { entropy_src, EntropySrcEsObserveFifoReady, EntropySrcInterrupt::EsObserveFifoReady },
    { entropy_src, EntropySrcEsFatalErr, EntropySrcInterrupt::EsFatalErr },

    // EDN0 interrupts
    { edn0, Edn0EdnCmdReqDone, EdnInterrupt::EdnCmdReqDone },
    { edn0, Edn0EdnFatalErr, EdnInterrupt::EdnFatalErr },

    // EDN1 interrupts
    { edn1, Edn1EdnCmdReqDone, EdnInterrupt::EdnCmdReqDone },
    { edn1, Edn1EdnFatalErr, EdnInterrupt::EdnFatalErr },
];
