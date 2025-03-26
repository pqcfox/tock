// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2023.

use earlgrey::pinmux::MuxedPadsWrapper;
use earlgrey::pinmux_config::{EarlGreyPinmuxConfig, INPUT_NUM, OUTPUT_NUM};
use earlgrey::registers::top_earlgrey::{PinmuxInsel, PinmuxOutsel};

type In = PinmuxInsel;
type Out = PinmuxOutsel;

/// Pinmux configuration for hyperdebug with cw310. The primary reference for
/// this config is:
/// <OPENTITAN_TREE>/hw/top_earlgrey/data/pins_cw310_hyperdebug.xdc
/// pins_cw310_hyperdebug.xdc sometimes underspecifies the layout (e.g. it
/// doesn't indicate UART numbering); we resolve the ambiguity by using
/// <OPENTITAN_TREE>/hw/top_earlgrey/data/pins_cw310.xdc
pub enum BoardPinmuxLayout {}

impl EarlGreyPinmuxConfig for BoardPinmuxLayout {
    #[rustfmt::skip]
    const INPUT: &'static [PinmuxInsel; INPUT_NUM] = &[
        In::Ioa2,         // GpioGpio0
        In::Ioa3,         // GpioGpio1
        In::Ioa6,         // GpioGpio2
        In::Iob0,         // GpioGpio3
        In::Iob1,         // GpioGpio4
        In::Iob2,         // GpioGpio5
        In::Iob3,         // GpioGpio6
        In::Iob6,         // GpioGpio7
        In::Iob7,         // GpioGpio8
        In::Iob8,         // GpioGpio9
        In::Ioc0,         // GpioGpio10
        In::Ioc1,         // GpioGpio11
        In::Ioc2,         // GpioGpio12
        In::Ioc5,         // GpioGpio13
        In::Ioc6,         // GpioGpio14
        In::ConstantZero, // GpioGpio15
        In::Ioc8,         // GpioGpio16
        In::Ioc9,         // GpioGpio17
        In::Ioc10,        // GpioGpio18
        In::Ioc11,        // GpioGpio19
        In::Ioc12,        // GpioGpio20
        In::Ior0,         // GpioGpio21
        In::Ior1,         // GpioGpio22
        In::Ior2,         // GpioGpio23
        In::Ior3,         // GpioGpio24
        In::Ior4,         // GpioGpio25
        In::Ior5,         // GpioGpio26
        In::Ior6,         // GpioGpio27
        In::Ior7,         // GpioGpio28
        In::Ior10,        // GpioGpio29
        In::Ior11,        // GpioGpio30
        In::Ior12,        // GpioGpio31
        In::Iob12,        // I2c0Sda
        In::Iob11,        // I2c0Scl
        In::Ioa7,         // I2c1Sda
        In::Ioa8,         // I2c1Scl
        In::Iob9,         // I2c2Sda
        In::Iob10,        // I2c2Scl
        In::ConstantZero, // SpiHost1Sd0
        In::ConstantZero, // SpiHost1Sd1
        In::ConstantZero, // SpiHost1Sd2
        In::ConstantZero, // SpiHost1Sd3
        In::Ioc3,         // Uart0Rx
        In::Iob4,         // Uart1Rx
        In::Ioa0,         // Uart2Rx
        In::Ioa4,         // Uart3Rx
        In::ConstantZero, // SpiDeviceTpmCsb
        In::ConstantZero, // FlashCtrlTck
        In::ConstantZero, // FlashCtrlTms
        In::ConstantZero, // FlashCtrlTdi
        In::ConstantZero, // SysrstCtrlAonAcPresent
        In::ConstantZero, // SysrstCtrlAonKey0In
        In::ConstantZero, // SysrstCtrlAonKey1In
        In::ConstantZero, // SysrstCtrlAonKey2In
        In::ConstantZero, // SysrstCtrlAonPwrbIn
        In::ConstantZero, // SysrstCtrlAonLidOpen
        In::Ioc7,         // UsbdevSense
    ];

    #[rustfmt::skip]
    const OUTPUT: &'static [PinmuxOutsel; OUTPUT_NUM] = &[
        // __________  BANK IOA __________
        Out::ConstantHighZ, // Ioa0 UART2_RX
        Out::Uart2Tx,       // Ioa1 UART2_TX
        Out::GpioGpio0,     // Ioa2
        Out::GpioGpio1,     // Ioa3
        Out::ConstantHighZ, // Ioa4 UART3_RX
        Out::Uart3Tx,       // Ioa5 UART3_TX
        Out::GpioGpio2,     // Ioa6
        Out::I2c1Sda,       // Ioa7 I2C1_SDA
        Out::I2c1Scl,       // Ioa8 I2C1_SCL
        // __________ BANK IOB __________
        Out::GpioGpio3,     // Iob0 SPI_HOST_CS
        Out::GpioGpio4,     // Iob1 SPI_HOST_DI
        Out::GpioGpio5,     // Iob2 SPI_HOST_DO
        Out::GpioGpio6,     // Iob3 SPI_HOST_CLK
        Out::ConstantHighZ, // Iob4 UART1_RX
        Out::Uart1Tx,       // Iob5 UART1_TX
        Out::GpioGpio7,     // Iob6
        Out::GpioGpio8,     // Iob7
        Out::GpioGpio9,     // Iob8
        Out::I2c2Sda,       // Iob9  I2C2_SDA
        Out::I2c2Scl,       // Iob10 I2C2_SCL
        Out::I2c0Scl,       // Iob11 I2C0_SCL
        Out::I2c0Sda,       // Iob12 I2C0_SDA
        // __________ BANK IOC __________
        Out::GpioGpio10,    // Ioc0
        Out::GpioGpio11,    // Ioc1
        Out::GpioGpio12,    // Ioc2
        Out::ConstantHighZ, // Ioc3 UART0_RX
        Out::Uart0Tx,       // Ioc4 UART0_TX
        Out::ConstantHighZ, // Ioc5 (TAP STRAP 1)
        Out::GpioGpio14,    // Ioc6
        Out::ConstantHighZ, // Ioc7
        Out::ConstantHighZ, // Ioc8 (TAP STRAP 0)
        Out::GpioGpio17,    // Ioc9
        Out::GpioGpio18,    // Ioc10
        Out::GpioGpio19,    // Ioc11
        Out::GpioGpio20,    // Ioc12
        // __________ BANK IOR __________
        Out::GpioGpio21,    // Ior0
        Out::GpioGpio22,    // Ior1
        Out::GpioGpio23,    // Ior2
        Out::GpioGpio24,    // Ior3
        Out::GpioGpio25,    // Ior4
        Out::GpioGpio26,    // Ior5
        Out::GpioGpio27,    // Ior6
        Out::GpioGpio28,    // Ior7
        // DIO CW310_hyp    // Ior8
        // DIO CW310_hyp    // Ior9
        Out::GpioGpio29,    // Ior10
        Out::GpioGpio30,    // Ior11
        Out::GpioGpio31,    // Ior12
        Out::ConstantHighZ, // Ior13
    ];
}

/// prepare pinmux and gpio registers for testing of SysRstr_Ctrl
pub fn prepare_wiring_sysrst_ctrl_tests() {
    use earlgrey::pinmux::{PadConfig, SelectInput, SelectOutput};
    use earlgrey::registers::top_earlgrey::MuxedPads;
    use earlgrey::registers::top_earlgrey::PinmuxPeripheralIn;
    use kernel::hil::gpio::Configure;
    use lowrisc::gpio::GpioPin;
    // in order to test sysrst_ctrl 3 wires need to be connected on CW310 board
    // these 3 wires will connect GPIO Outputs to SysRst_Ctrl's inputs and a GPIO Input to one of SysRstr's output such that the test functions are able to trigger various input combinations and observe one of the output signals

    kernel::debug!("SystemReset_Ctrl wiring setup: ");
    kernel::debug!("  Connect PMOD1_1 to PMOD1_5 using a wire");
    kernel::debug!("  Connect PMOD1_2 to PMOD1_6 using a wire");
    kernel::debug!("  Connect PMOD1_3 to PMOD1_7 using a wire");
    kernel::debug!("SystemReset_Ctrl's test STARTED");

    // prepare IOs for SysRstCtrl tests
    //     GPIO 2           PAD (A6)           PAD (A2)      SysRst
    // key0_force_gpio ->  key0_force -wire-> key0_input -> key0_input
    //
    //     GPIO 20           PAD (C12)          PAD (A5)      SysRst
    // pwrb_force_gpio ->  pwrb_force -wire-> pwrb_input -> pwrb_input
    //
    //    SysRST       PAD (A4)           PAD (A8)      GPIO 7
    // key0_output ->  key0_out  -wire-> key0_sense -> key0_sense

    // on CW310 the wires are routed as:
    //  (key0_input) A2 -> PMOD_IO1 -wire-> PMOD_IO5 A6  (key0_force)
    //  (pwrb_input) A5 -> PMOD_IO2 -wire-> PMOD_IO6 C12 (pwrb_force)
    //  (key0_out)   A5 -> PMOD_IO3 -wire-> PMOD_IO7 A8  (key0_sense)
    let key0_force = MuxedPads::Ioa6; // Gpio2 output
    let key0_input = MuxedPads::Ioa2; // SysRstCtrl.key0_input

    let key0_out = MuxedPads::Ioa4; // SysRstCtrl.key0_output
    let key0_sense = MuxedPads::Ioa8; // Gpio7 input

    let pwrb_force = MuxedPads::Ioc12; // Gpio20 output
    let pwrb_input = MuxedPads::Ioa5; // SysRstCtrl.pwrb_input

    key0_out.connect_output(PinmuxOutsel::SysrstCtrlAonKey0Out);

    let key0_force_gpio = GpioPin::new(
        earlgrey::gpio::GPIO_BASE,
        PadConfig::InOut(
            key0_force,
            PinmuxPeripheralIn::GpioGpio2,
            PinmuxOutsel::GpioGpio2,
        ),
        2,
    );

    let pwrb_force_gpio = GpioPin::new(
        earlgrey::gpio::GPIO_BASE,
        PadConfig::InOut(
            pwrb_force,
            PinmuxPeripheralIn::GpioGpio20,
            PinmuxOutsel::GpioGpio20,
        ),
        20,
    );

    key0_force_gpio.make_output();
    pwrb_force_gpio.make_output();

    // configure each input pin as HiZ + connectedt to SysRstCtrl/GPIO
    // code below should work but it doesn't (left it here as it's intent is much more readable)
    // PadConfig::InOut(
    //     key0_input,
    //     PinmuxPeripheralIn::SysrstCtrlAonKey0In,
    //     PinmuxOutsel::ConstantHighZ,
    // )
    // .connect();

    // PadConfig::InOut(
    //     pwrb_input,
    //     PinmuxPeripheralIn::SysrstCtrlAonPwrbIn,
    //     PinmuxOutsel::ConstantHighZ,
    // )
    // .connect();

    // PadConfig::InOut(
    //     key0_sense,
    //     PinmuxPeripheralIn::GpioGpio7,
    //     PinmuxOutsel::ConstantHighZ,
    // )
    // .connect();

    // this code should do exactly the same thing as the code above but this one works
    PinmuxPeripheralIn::SysrstCtrlAonKey0In
        .connect_input(PinmuxInsel::from(MuxedPadsWrapper(key0_input)));
    key0_input.connect_output(PinmuxOutsel::ConstantHighZ);

    PinmuxPeripheralIn::SysrstCtrlAonPwrbIn
        .connect_input(PinmuxInsel::from(MuxedPadsWrapper(pwrb_input)));
    pwrb_input.connect_output(PinmuxOutsel::ConstantHighZ);

    PinmuxPeripheralIn::GpioGpio7.connect_input(PinmuxInsel::from(MuxedPadsWrapper(key0_sense)));
    key0_sense.connect_output(PinmuxOutsel::ConstantHighZ);

    // check that the pins have been correctly routed
    assert!(key0_force.get_selector() == PinmuxOutsel::GpioGpio2);
    assert!(pwrb_force.get_selector() == PinmuxOutsel::GpioGpio20);
    assert!(key0_input.get_selector() == PinmuxOutsel::ConstantHighZ);
    assert!(pwrb_input.get_selector() == PinmuxOutsel::ConstantHighZ);
    assert!(key0_out.get_selector() == PinmuxOutsel::SysrstCtrlAonKey0Out);
    assert!(key0_sense.get_selector() == PinmuxOutsel::ConstantHighZ);

    assert!(
        PinmuxPeripheralIn::SysrstCtrlAonKey0In.get_selector()
            == MuxedPadsWrapper(key0_input).into()
    );
    assert!(
        PinmuxPeripheralIn::SysrstCtrlAonPwrbIn.get_selector()
            == MuxedPadsWrapper(pwrb_input).into()
    );
    assert!(PinmuxPeripheralIn::GpioGpio7.get_selector() == MuxedPadsWrapper(key0_sense).into());
    assert!(PinmuxPeripheralIn::GpioGpio2.get_selector() == MuxedPadsWrapper(key0_force).into());
}
