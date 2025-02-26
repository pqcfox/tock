// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use crate::{config, items, state, Opts};
use parse::DefaultPeripherals;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

/// Decoded JSON capsule configuration
#[derive(Deserialize)]
struct CapsuleConfig {
    name: String,
    #[serde(default)]
    attrs: HashMap<String, String>,
}

/// Resolves a value conflict by taking the largest value. Useful for ensuring a buffer is long
/// enough for all users.
fn resolve_conflict_greatest<'a, T: Ord>(a: &'a T, b: &'a T) -> Result<&'a T, ()> {
    if a >= b {
        Ok(a)
    } else {
        Ok(b)
    }
}

/// Resolves a value conflict by requiring that both values match. If not, the configuration is not
/// valid. This is used for global parameters that must be agreed upon exactly everywhere, such as
/// the UART baud rate.
fn resolve_conflict_must_match<'a, T: Eq>(a: &'a T, b: &'a T) -> Result<&'a T, ()> {
    if a == b {
        Ok(a)
    } else {
        Err(())
    }
}

/// Macro that processes an input for a capsule that takes no parameters.
macro_rules! capsule_no_fields {
    ($capsule_config:ident, $data:ident, $peripheral:ident, $capsule_sc:ident, $capsule_uc:ident, $update:ident) => {{
        // Get the peripheral
        let $peripheral = match $data.chip.peripherals().$peripheral() {
            // TODO: Support selecting from multiple peripherals in `attrs`.
            Ok(peripherals) => peripherals[0].clone(),
            Err(_) => panic!(concat!(
                "Selected chip does not have a peripheral that supports the ",
                stringify!($capsule_sc),
                " capsule."
            )),
        };
        $data.platform.$update($peripheral.clone());
    }};
}

/// Macro that processes an input for a capsule that takes a single parameter. The `$resolver`
/// parameter dictates how different field values required by different config files is
/// disambiguated.
macro_rules! capsule_single_field {
    ($capsule_config:ident, $data:ident, $peripheral:ident, $capsule_sc:ident, $capsule_uc:ident, $field:ident, $field_ty:ty, $update:ident, $resolver:ident) => {{
        // Get the field value from the config JSON.
        let $field = $capsule_config
            .attrs
            .get(stringify!($field))
            .expect(concat!(
                "Attribute `",
                stringify!($field),
                "` missing on `",
                stringify!($capsule_sc),
                "` capsule."
            ));
        let $field = str::parse::<$field_ty>($field).expect(concat!(
            "Could not parse `",
            stringify!($field),
            "` as a `",
            stringify!($field_ty),
            "`."
        ));
        // Get the peripheral
        let mut opt = $data.platform.capsule(&config::Index::$capsule_uc);
        let $peripheral = match $data.chip.peripherals().$peripheral() {
            // TODO: Support selecting from multiple peripherals in `attrs`.
            Ok(peripherals) => peripherals[0].clone(),
            Err(_) => panic!(concat!(
                "Selected chip does not have a peripheral that supports the ",
                stringify!($capsule_sc),
                " capsule."
            )),
        };
        // Disambiguate between the new and existing field value, if necessary.
        let existing = match opt.get_or_insert(&crate::config::Capsule::$capsule_sc {
            $peripheral: $peripheral.clone(),
            $field,
        }) {
            crate::config::Capsule::$capsule_sc { $field, .. } => *$field,
            _ => unreachable!("Wrong capsule type set. Something went wrong."),
        };
        let disambiguated = match $resolver(&existing, &$field) {
            Ok(val) => val,
            Err(()) => panic!(
                concat!(
                    "Conflicting values for ",
                    stringify!($capsule_sc),
                    " field `",
                    stringify!($field),
                    "`: `{}` and `{}`"
                ),
                existing, $field,
            ),
        };
        $data.platform.$update($peripheral.clone(), *disambiguated);
    }};
}

fn label_to_pin_id(label: &str) -> Option<usize> {
    let pin_name = label.to_lowercase();
    pin_name
        .strip_prefix("pin")
        .map(|pin_num| str::parse::<usize>(pin_num).ok())?
}

macro_rules! process_gpio_capsule {
    ($data:ident, $capsule_config:ident, $capsule:ident) => {{
        let gpio = match $data.chip.peripherals().gpio() {
            // TODO: Support selecting from multiple peripherals in `attrs`.
            Ok(peripherals) => peripherals[0].clone(),
            Err(_) => panic!(concat!(
                "Selected chip does not have a peripheral that supports the `",
                stringify!($capsule),
                "` capsule.",
            )),
        };
        for (pin, setting) in &$capsule_config.attrs {
            if let Some(pin_id) = label_to_pin_id(&pin) {
                let pins = $data.gpio(&gpio).unwrap().pins();
                let (pin, existing) = match pins.get(pin_id) {
                    Some(p) => p,
                    // Selected pin out of range, ignore.
                    None => continue,
                };
                if setting == "on" {
                    // Check the pin is not already in use for another purpose.
                    match existing {
                        state::PinFunction::None | state::PinFunction::$capsule => {}
                        other => panic!(
                            "GPIO pin #{} cannot be assigned to both {:?} and {:?}",
                            pin_id,
                            other,
                            state::PinFunction::$capsule,
                        ),
                    }
                    $data.change_pin_status(gpio.clone(), *pin, state::PinFunction::$capsule);
                } else if setting != "off" {
                    panic!(
                        "Invalid value for GPIO pin setting: `{}`. Valid values are `on` or `off`.",
                        setting
                    );
                }
            }
        }
    }};
}

/// Check that LLDB and Console haven't selected different baud rates.
fn check_baud_rate<C: parse::Chip>(data: &mut state::Data<C>) {
    let console_baudrate = match data.platform.capsule(&config::Index::CONSOLE) {
        Some(crate::config::Capsule::Console { baud_rate, .. }) => Some(baud_rate),
        _ => None,
    };
    let lldb_baudrate = match data.platform.capsule(&config::Index::LLDB) {
        Some(crate::config::Capsule::Lldb { baud_rate, .. }) => Some(baud_rate),
        _ => None,
    };
    if let Some(console_baudrate) = console_baudrate {
        if let Some(lldb_baudrate) = lldb_baudrate {
            if console_baudrate != lldb_baudrate {
                panic!(
                    "Cannot use different UART baud rates for `Console` ({}) and `LLDB` ({}).",
                    console_baudrate, lldb_baudrate,
                )
            }
        }
    }
}

/// Adds the capsule configuration to the platform.
///
/// # Panics
///
/// - If `capsule_config.name` contains an unsupported capsule name.
/// - If `capsule_config.attrs` does not contain a required capsule parameter.
/// - If the selected chip does not support the capsule.
/// - If an integer attribute cannot be parsed as an integer.
/// - If the configuration conflicts with another capsule already set on the
///   platform, such as AES
///   with two different block sizes.
fn process_capsule_config<C: parse::Chip>(
    data: &mut state::Data<C>,
    capsule_config: &CapsuleConfig,
) {
    match capsule_config.name.to_lowercase().as_str() {
        "aes" => capsule_single_field!(
            capsule_config,
            data,
            aes,
            Aes,
            AES,
            number_of_blocks,
            usize,
            update_aes,
            resolve_conflict_greatest
        ),
        "alarm" => capsule_no_fields!(capsule_config, data, timer, Alarm, ALARM, update_alarm),
        "alert_handler" => capsule_no_fields!(
            capsule_config,
            data,
            alert_handler,
            AlertHandler,
            ALERT_HANDLER,
            update_alert_handler
        ),
        "console" => {
            capsule_single_field!(
                capsule_config,
                data,
                uart,
                Console,
                CONSOLE,
                baud_rate,
                usize,
                update_console,
                resolve_conflict_must_match
            );
            check_baud_rate(data);
        }
        "flash" => capsule_single_field!(
            capsule_config,
            data,
            flash,
            Flash,
            FLASH,
            buffer_size,
            usize,
            update_flash,
            resolve_conflict_greatest
        ),
        "gpio" => {
            process_gpio_capsule!(data, capsule_config, Gpio);
            let helper = &data
                .gpio_list
                .as_ref()
                .unwrap()
                .iter()
                // TODO: support more than one peripheral for GPIO
                .next()
                .expect("No available GPIO peripheral on the selected chip.");
            let selected_pins = helper
                .pins
                .iter()
                .filter(|(_, pin_function)| matches!(pin_function, state::PinFunction::Gpio))
                .map(|(pin_id, _)| *pin_id)
                .collect();
            data.platform.update_gpio(selected_pins);
        }
        "hmac" => capsule_single_field!(
            capsule_config,
            data,
            hmac,
            Hmac,
            HMAC,
            length,
            usize,
            update_hmac,
            resolve_conflict_greatest
        ),
        "i2c" => capsule_no_fields!(capsule_config, data, i2c, I2c, I2C, update_i2c),
        "ipc" => data.platform.update_ipc(),
        "info_flash" => capsule_no_fields!(
            capsule_config,
            data,
            flash,
            InfoFlash,
            INFO_FLASH,
            update_info_flash
        ),
        "kv_driver" => capsule_no_fields!(
            capsule_config,
            data,
            flash,
            KvDriver,
            KV_DRIVER,
            update_kv_driver
        ),
        "lldb" => {
            capsule_single_field!(
                capsule_config,
                data,
                uart,
                Lldb,
                LLDB,
                baud_rate,
                usize,
                update_lldb,
                resolve_conflict_must_match
            );
            check_baud_rate(data);
        }
        "pattgen" => capsule_no_fields!(
            capsule_config,
            data,
            pattgen,
            Pattgen,
            PATTGEN,
            update_pattgen
        ),
        "reset_manager" => capsule_no_fields!(
            capsule_config,
            data,
            reset_manager,
            ResetManager,
            RESET_MANAGER,
            update_reset_manager
        ),
        "rng" => capsule_no_fields!(capsule_config, data, rng, Rng, RNG, update_rng),
        "spi" => capsule_no_fields!(capsule_config, data, spi, Spi, SPI, update_spi),
        "system_reset_controller" => capsule_no_fields!(
            capsule_config,
            data,
            system_reset_controller,
            SystemResetController,
            SYSTEM_RESET_CONTROLLER,
            update_system_reset_controller
        ),
        "temperature" => capsule_no_fields!(
            capsule_config,
            data,
            temp,
            Temperature,
            TEMPERATURE,
            update_temp
        ),
        "usb" => capsule_no_fields!(capsule_config, data, usb, Usb, USB, update_usb),
        "oneshot_digest" => capsule_no_fields!(
            capsule_config,
            data,
            oneshot_digest,
            ONESHOT_DIGEST,
            ONESHOT_DIGEST,
            update_oneshot_digest
        ),
        "p256" => capsule_no_fields!(capsule_config, data, p256, P256, P256, update_p256),
        "p384" => capsule_no_fields!(capsule_config, data, p384, P384, P384, update_p384),
        "attestation" => capsule_no_fields!(
            capsule_config,
            data,
            attestation,
            ATTESTATION,
            ATTESTATION,
            update_attestation
        ),
        _ => panic!(
            "Unsupported capsule name `{}` in capsule config file.",
            capsule_config.name
        ),
    }
}

/// Runs the configurator without a GUI based on the command-line arguments.
pub fn run_cli_mode(mut opts: Opts) {
    // Parse capsule files
    let mut capsules = vec![];
    for capsule_list in &opts.capsules {
        let paths = capsule_list.split_whitespace();
        for path in paths {
            // Open the file
            let mut file = File::open(path)
                .unwrap_or_else(|_| panic!("Could not open capsule file `{}`", path));
            let len = file
                .metadata()
                .unwrap_or_else(|_| panic!("Could not read length of capsule file `{}`", path))
                .len();
            // Create a buffer for the JSON equal to the file's length.
            let mut buf =
                String::with_capacity(usize::try_from(len).unwrap_or_else(|_| {
                    panic!("Capsule file `{}` is too long for a `usize`.", path)
                }));
            // Read the JSON from the file to the buffer.
            file.read_to_string(&mut buf)
                .unwrap_or_else(|_| panic!("Could not read capsule file `{}`", path));
            // Parse the JSON into a `CapsuleConfig`.
            let capsule_config: CapsuleConfig = serde_json::from_str(&buf)
                .unwrap_or_else(|_| panic!("Unexpected format in capsule file `{}`", path));
            capsules.push(capsule_config);
        }
    }
    let mut data = run_cli_inner(&mut opts, &capsules);
    // Write JSON output
    if let Some(out) = opts.out {
        data.set_out(out);
    }
    state::write_json(&mut data);
}

fn run_cli_inner(opts: &mut Opts, capsules: &[CapsuleConfig]) -> state::Data<lowrisc::Chip> {
    let mut data = state::Data::new(match opts.chip {
        items::SupportedChip::EarlgreyCw310 => lowrisc::Chip::new(),
    });

    // Record platform name
    data.platform
        .update_type(std::mem::take(&mut opts.platform));
    // Record stack size
    data.platform.update_stack_size(opts.stack_size);
    // Record scheduler
    data.platform.update_scheduler(opts.scheduler);
    // Record syscall filter
    data.platform.update_syscall_filter(opts.syscall_filter);
    // Record process count
    data.platform.process_count = opts.processes;
    // Parse capsule data
    for capsule_config in capsules {
        process_capsule_config(&mut data, capsule_config);
    }
    data
}

#[cfg(test)]
mod tests {
    use crate::cli::{run_cli_inner, CapsuleConfig};
    use crate::items::SupportedChip;
    use crate::{Mode, Opts};
    use std::collections::HashMap;

    /// Checks the configurator outputs the expected JSON configuration format
    /// in a basic case with no capsules.
    #[test]
    fn test_basic() {
        let mut opts = Opts {
            mode: Mode::Cli,
            chip: SupportedChip::EarlgreyCw310,
            platform: String::from("AutogeneratedPlatform"),
            stack_size: 2000,
            scheduler: parse::SchedulerType::Cooperative,
            syscall_filter: parse::SyscallFilterType::None,
            processes: 0,
            capsules: vec![],
            out: None,
        };
        let expected = r#"{
  "TYPE": "AutogeneratedPlatform",
  "CAPSULES": {},
  "SCHEDULER": "Cooperative",
  "PROCESS_COUNT": 0,
  "STACK_SIZE": 2000,
  "SYSCALL_FILTER": "None"
}"#;
        let data = run_cli_inner(&mut opts, &[]);
        let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
        assert_eq!(
            expected,
            board_config.as_str(),
            "Board config did not match expected value"
        )
    }

    /// Checks the configurator respects the specified platform name.
    #[test]
    fn test_platform_name() {
        let mut opts = Opts {
            mode: Mode::Cli,
            chip: SupportedChip::EarlgreyCw310,
            platform: String::from("TestPlatformName"),
            stack_size: 2000,
            scheduler: parse::SchedulerType::Cooperative,
            syscall_filter: parse::SyscallFilterType::None,
            processes: 0,
            capsules: vec![],
            out: None,
        };
        let expected = r#"{
  "TYPE": "TestPlatformName",
  "CAPSULES": {},
  "SCHEDULER": "Cooperative",
  "PROCESS_COUNT": 0,
  "STACK_SIZE": 2000,
  "SYSCALL_FILTER": "None"
}"#;
        let data = run_cli_inner(&mut opts, &[]);
        let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
        assert_eq!(
            expected,
            board_config.as_str(),
            "Board config did not match expected value"
        )
    }

    /// Checks the configurator respects the specified stack size.
    #[test]
    fn test_stack_size() {
        let mut opts = Opts {
            mode: Mode::Cli,
            chip: SupportedChip::EarlgreyCw310,
            platform: String::from("AutogeneratedPlatform"),
            stack_size: usize::try_from(u32::MAX).unwrap(),
            scheduler: parse::SchedulerType::Cooperative,
            syscall_filter: parse::SyscallFilterType::None,
            processes: 0,
            capsules: vec![],
            out: None,
        };
        let expected = r#"{
  "TYPE": "AutogeneratedPlatform",
  "CAPSULES": {},
  "SCHEDULER": "Cooperative",
  "PROCESS_COUNT": 0,
  "STACK_SIZE": 4294967295,
  "SYSCALL_FILTER": "None"
}"#;
        let data = run_cli_inner(&mut opts, &[]);
        let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
        assert_eq!(
            expected,
            board_config.as_str(),
            "Board config did not match expected value"
        )
    }
    /// Checks the configurator respects the specified scheduler.
    #[test]
    fn test_scheduler() {
        let mut opts = Opts {
            mode: Mode::Cli,
            chip: SupportedChip::EarlgreyCw310,
            platform: String::from("AutogeneratedPlatform"),
            stack_size: 2000,
            scheduler: parse::SchedulerType::RoundRobin,
            syscall_filter: parse::SyscallFilterType::None,
            processes: 0,
            capsules: vec![],
            out: None,
        };
        let expected = r#"{
  "TYPE": "AutogeneratedPlatform",
  "CAPSULES": {},
  "SCHEDULER": "RoundRobin",
  "PROCESS_COUNT": 0,
  "STACK_SIZE": 2000,
  "SYSCALL_FILTER": "None"
}"#;
        let data = run_cli_inner(&mut opts, &[]);
        let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
        assert_eq!(
            expected,
            board_config.as_str(),
            "Board config did not match expected value"
        )
    }
    /// Checks the configurator respects the specified syscall filter type.
    #[test]
    fn test_syscall_filter_type() {
        let mut opts = Opts {
            mode: Mode::Cli,
            chip: SupportedChip::EarlgreyCw310,
            platform: String::from("AutogeneratedPlatform"),
            stack_size: 2000,
            scheduler: parse::SchedulerType::Cooperative,
            syscall_filter: parse::SyscallFilterType::TbfHeaderFilterDefaultAllow,
            processes: 0,
            capsules: vec![],
            out: None,
        };
        let expected = r#"{
  "TYPE": "AutogeneratedPlatform",
  "CAPSULES": {},
  "SCHEDULER": "Cooperative",
  "PROCESS_COUNT": 0,
  "STACK_SIZE": 2000,
  "SYSCALL_FILTER": "TbfHeaderFilterDefaultAllow"
}"#;
        let data = run_cli_inner(&mut opts, &[]);
        let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
        assert_eq!(
            expected,
            board_config.as_str(),
            "Board config did not match expected value"
        )
    }

    /// Checks the configurator respects the specified process count.
    #[test]
    fn test_process_count() {
        let mut opts = Opts {
            mode: Mode::Cli,
            chip: SupportedChip::EarlgreyCw310,
            platform: String::from("AutogeneratedPlatform"),
            stack_size: 2000,
            scheduler: parse::SchedulerType::Cooperative,
            syscall_filter: parse::SyscallFilterType::None,
            processes: usize::try_from(u32::MAX).unwrap(),
            capsules: vec![],
            out: None,
        };
        let expected = r#"{
  "TYPE": "AutogeneratedPlatform",
  "CAPSULES": {},
  "SCHEDULER": "Cooperative",
  "PROCESS_COUNT": 4294967295,
  "STACK_SIZE": 2000,
  "SYSCALL_FILTER": "None"
}"#;
        let data = run_cli_inner(&mut opts, &[]);
        let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
        assert_eq!(
            expected,
            board_config.as_str(),
            "Board config did not match expected value"
        )
    }

    macro_rules! test_capsule_no_peripheral {
        {$test:ident, $capsule:expr, $capsule_uc:expr} => {
            /// Checks the configurator generates the correct platform
            /// configuration for capsules with no associated driver.
            #[test]
            fn $test() {
                let capsule_config = CapsuleConfig {
                    name: String::from($capsule),
                    attrs: HashMap::new(),
                };
                let mut opts = Opts {
                    mode: Mode::Cli,
                    chip: SupportedChip::EarlgreyCw310,
                    platform: String::from("AutogeneratedPlatform"),
                    stack_size: 2000,
                    scheduler: parse::SchedulerType::Cooperative,
                    syscall_filter: parse::SyscallFilterType::None,
                    processes: 0,
                    // Placeholder; we initialize the capsule config directly.
                    capsules: vec![],
                    out: None,
                };
                let expected = concat!(r#"{
  "TYPE": "AutogeneratedPlatform",
  "CAPSULES": {
    ""#, $capsule_uc, r#"": {
      "type": ""#, $capsule_uc, r#""
    }
  },
  "SCHEDULER": "Cooperative",
  "PROCESS_COUNT": 0,
  "STACK_SIZE": 2000,
  "SYSCALL_FILTER": "None"
}"#);
                let data = run_cli_inner(&mut opts, &[capsule_config]);
                let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
                assert_eq!(expected, board_config.as_str(), "Board config did not match expected value")
            }
        }
    }
    test_capsule_no_peripheral! {test_ipc, "ipc", "IPC"}

    macro_rules! test_capsule_no_fields {
        {$test:ident, $capsule:expr, $capsule_uc:expr, $peripheral:expr, $peripheral_type:expr} => {
            /// Checks the configurator generates the correct platform
            /// configuration for capsules that take no parameters.
            #[test]
            fn $test() {
                let capsule_config = CapsuleConfig {
                    name: String::from($capsule),
                    attrs: HashMap::new(),
                };
                let mut opts = Opts {
                    mode: Mode::Cli,
                    chip: SupportedChip::EarlgreyCw310,
                    platform: String::from("AutogeneratedPlatform"),
                    stack_size: 2000,
                    scheduler: parse::SchedulerType::Cooperative,
                    syscall_filter: parse::SyscallFilterType::None,
                    processes: 0,
                    // Placeholder; we initialize the capsule config directly.
                    capsules: vec![],
                    out: None,
                };
                let expected =concat!(r#"{
  "TYPE": "AutogeneratedPlatform",
  "CAPSULES": {
    ""#, $capsule_uc, r#"": {
      "type": ""#, $capsule_uc, r#"",
      ""#, $peripheral, r#"": "#, $peripheral_type, r#"
    }
  },
  "SCHEDULER": "Cooperative",
  "PROCESS_COUNT": 0,
  "STACK_SIZE": 2000,
  "SYSCALL_FILTER": "None"
}"#);
                let data = run_cli_inner(&mut opts, &[capsule_config]);
                let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
                assert_eq!(expected, board_config.as_str(), "Board config did not match expected value")
            }
        }
    }
    test_capsule_no_fields! {test_alarm, "alarm", "ALARM", "timer", "{}"}
    test_capsule_no_fields! {test_i2c, "i2c", "I2C", "i2c", "{}"}
    test_capsule_no_fields! {test_info_flash, "info_flash", "INFO_FLASH", "flash", "{}"}
    test_capsule_no_fields! {test_kv_driver, "kv_driver", "KV_DRIVER", "flash", "{}"}
    test_capsule_no_fields! {test_pattgen, "pattgen", "PATTGEN", "pattgen", "{}"}
    test_capsule_no_fields! {test_reset_manager, "reset_manager", "RESET_MANAGER", "reset_manager", "{}"}
    test_capsule_no_fields! {test_rng, "rng", "RNG", "rng", "{}"}
    test_capsule_no_fields! {test_spi, "spi", "SPI", "spi", "{}"}
    test_capsule_no_fields! {test_system_reset_controller, "system_reset_controller", "SYSTEM_RESET_CONTROLLER", "system_reset_controller", "{}"}
    test_capsule_no_fields! {test_usb, "usb", "USB", "usb", "{}"}
    test_capsule_no_fields! {test_oneshot_digest, "oneshot_digest", "ONESHOT_DIGEST", "oneshot_digest", "{}"}
    test_capsule_no_fields! {test_p256, "p256", "P256", "p256", r#"{
        "mux": {
          "timeout_mux": {
            "mux_alarm": {
              "peripheral": {}
            }
          }
        }
      }"#}
    test_capsule_no_fields! {test_p384, "p384", "P384", "p384", r#"{
        "mux": {
          "timeout_mux": {
            "mux_alarm": {
              "peripheral": {}
            }
          }
        }
      }"#}
    test_capsule_no_fields! {test_attestation, "attestation", "ATTESTATION", "attestation", r#"{
        "flash": {
          "peripheral": {}
        }
      }"#}

    // Tests for capsules with a single field.
    macro_rules! test_capsule_single_field {
        {$test_valid:ident, $test_missing_field:ident, $test_invalid_field_type:ident, $capsule:expr, $capsule_uc:expr, $peripheral:expr, $peripheral_type:expr, $field:expr, $field_value:expr} => {
            /// Checks the configurator generates the correct platform
            /// configuration for capsules that take a parameter.
            #[test]
            fn $test_valid() {
                let capsule_config = CapsuleConfig {
                    name: String::from($capsule),
                    attrs: HashMap::from([(String::from($field), String::from($field_value))]),
                };
                let mut opts = Opts {
                    mode: Mode::Cli,
                    chip: SupportedChip::EarlgreyCw310,
                    platform: String::from("AutogeneratedPlatform"),
                    stack_size: 2000,
                    scheduler: parse::SchedulerType::Cooperative,
                    syscall_filter: parse::SyscallFilterType::None,
                    processes: 0,
                    // Placeholder; we initialize the capsule config directly.
                    capsules: vec![],
                    out: None,
                };
                let expected =concat!(r#"{
  "TYPE": "AutogeneratedPlatform",
  "CAPSULES": {
    ""#, $capsule_uc, r#"": {
      "type": ""#, $capsule_uc, r#"",
      ""#, $peripheral, r#"": "#, $peripheral_type, r#",
      ""#, $field, r#"": "#, $field_value, r#"
    }
  },
  "SCHEDULER": "Cooperative",
  "PROCESS_COUNT": 0,
  "STACK_SIZE": 2000,
  "SYSCALL_FILTER": "None"
}"#);
                let data = run_cli_inner(&mut opts, &[capsule_config]);
                let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
                assert_eq!(expected, board_config.as_str(), "Board config did not match expected value")
            }

            /// Checks the configurator rejects a capsule configuration that is
            /// missing a required field.
            #[test]
            #[should_panic]
            fn $test_missing_field() {
                let capsule_config = CapsuleConfig {
                    name: String::from($capsule),
                    // Intentionally don't add the required field.
                    attrs: HashMap::new(),
                };
                let mut opts = Opts {
                    mode: Mode::Cli,
                    chip: SupportedChip::EarlgreyCw310,
                    platform: String::from("AutogeneratedPlatform"),
                    stack_size: 2000,
                    scheduler: parse::SchedulerType::Cooperative,
                    syscall_filter: parse::SyscallFilterType::None,
                    processes: 0,
                    // Placeholder; we initialize the capsule config directly.
                    capsules: vec![],
                    out: None,
                };
                // This should panic due to the missing field.
                let _ = run_cli_inner(&mut opts, &[capsule_config]);
            }

            /// Checks the configurator rejects a capsule configuration whose
            /// field value is not an integer when the capsule expects one.
            #[test]
            #[should_panic]
            fn $test_invalid_field_type() {
                let capsule_config = CapsuleConfig {
                    name: String::from($capsule),
                    // Intentionally set the field to a non-integer value.
                    attrs: HashMap::from([(String::from($field), String::from("not an integer"))]),
                };
                let mut opts = Opts {
                    mode: Mode::Cli,
                    chip: SupportedChip::EarlgreyCw310,
                    platform: String::from("AutogeneratedPlatform"),
                    stack_size: 2000,
                    scheduler: parse::SchedulerType::Cooperative,
                    syscall_filter: parse::SyscallFilterType::None,
                    processes: 0,
                    // Placeholder; we initialize the capsule config directly.
                    capsules: vec![],
                    out: None,
                };
                // This should panic due to the field value being a non-integer.
                let _ = run_cli_inner(&mut opts, &[capsule_config]);
            }
        }
    }
    test_capsule_single_field! {test_aes, test_aes_missing_field, test_aes_invalid_field_type, "aes", "AES", "aes", "{}", "number_of_blocks", "64"}
    test_capsule_single_field! {test_console, test_console_missing_field, test_console_invalid_field_type, "console", "CONSOLE", "uart", "\"Uart0\"", "baud_rate", "123456"}
    test_capsule_single_field! {test_flash, test_flash_missing_field, test_flash_invalid_field_type, "flash", "FLASH", "flash", "{}", "buffer_size", "2048"}
    test_capsule_single_field! {test_hmac, test_hmac_missing_field, test_hmac_invalid_field_type, "hmac", "HMAC", "hmac", "{}", "length", "32"}
    test_capsule_single_field! {test_lldb, test_lldb_missing_field, test_lldb_invalid_field_type, "lldb", "LLDB", "uart", "\"Uart0\"", "baud_rate", "123456"}

    #[test]
    fn test_gpio() {
        let capsule_config = CapsuleConfig {
            name: String::from("gpio"),
            attrs: HashMap::from([
                (String::from("pin0"), String::from("on")),
                (String::from("pin1"), String::from("on")),
                (String::from("pin2"), String::from("on")),
                (String::from("pin3"), String::from("on")),
                (String::from("pin4"), String::from("on")),
                (String::from("pin5"), String::from("on")),
                (String::from("pin6"), String::from("on")),
                (String::from("pin7"), String::from("on")),
                (String::from("pin8"), String::from("off")),
                (String::from("pin9"), String::from("off")),
                (String::from("pin10"), String::from("off")),
                (String::from("pin11"), String::from("off")),
                (String::from("pin12"), String::from("off")),
                (String::from("pin13"), String::from("off")),
                (String::from("pin14"), String::from("off")),
                (String::from("pin15"), String::from("off")),
            ]),
        };
        let mut opts = Opts {
            mode: Mode::Cli,
            chip: SupportedChip::EarlgreyCw310,
            platform: String::from("AutogeneratedPlatform"),
            stack_size: 2000,
            scheduler: parse::SchedulerType::Cooperative,
            syscall_filter: parse::SyscallFilterType::None,
            processes: 0,
            // Placeholder; we initialize the capsule config directly.
            capsules: vec![],
            out: None,
        };
        let expected = r#"{
  "TYPE": "AutogeneratedPlatform",
  "CAPSULES": {
    "GPIO": {
      "type": "GPIO",
      "pins": [
        "Pin0",
        "Pin1",
        "Pin2",
        "Pin3",
        "Pin4",
        "Pin5",
        "Pin6",
        "Pin7"
      ]
    }
  },
  "SCHEDULER": "Cooperative",
  "PROCESS_COUNT": 0,
  "STACK_SIZE": 2000,
  "SYSCALL_FILTER": "None"
}"#;
        let data = run_cli_inner(&mut opts, &[capsule_config]);
        let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
        assert_eq!(
            expected,
            board_config.as_str(),
            "Board config did not match expected value"
        )
    }

    /// Test that the GPIO capsule processing logic rejects invalid pin
    /// settings.
    #[test]
    #[should_panic]
    fn test_gpio_invalid_value() {
        let capsule_config = CapsuleConfig {
            name: String::from("gpio"),
            attrs: HashMap::from([
                // Intentionally set the pin to an invalid setting.
                (String::from("pin0"), String::from("not on or off")),
            ]),
        };
        let mut opts = Opts {
            mode: Mode::Cli,
            chip: SupportedChip::EarlgreyCw310,
            platform: String::from("AutogeneratedPlatform"),
            stack_size: 2000,
            scheduler: parse::SchedulerType::Cooperative,
            syscall_filter: parse::SyscallFilterType::None,
            processes: 0,
            // Placeholder; we initialize the capsule config directly.
            capsules: vec![],
            out: None,
        };
        // Should panic due to the invalid field value.
        let _ = run_cli_inner(&mut opts, &[capsule_config]);
    }

    macro_rules! test_capsule_field_resolution_greatest {
        {$test:ident, $capsule:expr, $capsule_uc:expr, $peripheral:expr, $peripheral_type:expr, $field:expr, $field_value_1:expr, $field_value_2:expr} => {
            /// Check that the configurator correctly resolves field value
            /// conflicts for capsules that should use the largest requested
            /// value.
            #[test]
            fn $test() {
                // By convention, this test must be configured such that
                // $field_value_1 < $field_value_2.
                let capsule_config_1 = CapsuleConfig {
                    name: String::from($capsule),
                    attrs: HashMap::from([(String::from($field), String::from($field_value_1))]),
                };
                let capsule_config_2 = CapsuleConfig {
                    name: String::from($capsule),
                    attrs: HashMap::from([(String::from($field), String::from($field_value_2))]),
                };
                let mut opts = Opts {
                    mode: Mode::Cli,
                    chip: SupportedChip::EarlgreyCw310,
                    platform: String::from("AutogeneratedPlatform"),
                    stack_size: 2000,
                    scheduler: parse::SchedulerType::Cooperative,
                    syscall_filter: parse::SyscallFilterType::None,
                    processes: 0,
                    // Placeholder; we initialize the capsule configs directly.
                    capsules: vec![],
                    out: None,
                };
                // Check that `$field_value_2` is chosen over `$field_value_1`.
                let expected =concat!(r#"{
  "TYPE": "AutogeneratedPlatform",
  "CAPSULES": {
    ""#, $capsule_uc, r#"": {
      "type": ""#, $capsule_uc, r#"",
      ""#, $peripheral, r#"": "#, $peripheral_type, r#",
      ""#, $field, r#"": "#, $field_value_2, r#"
    }
  },
  "SCHEDULER": "Cooperative",
  "PROCESS_COUNT": 0,
  "STACK_SIZE": 2000,
  "SYSCALL_FILTER": "None"
}"#);
                let data = run_cli_inner(&mut opts, &[capsule_config_1, capsule_config_2]);
                let board_config = serde_json::to_string_pretty(&data.platform).unwrap();
                assert_eq!(expected, board_config.as_str(), "Board config did not match expected value")
            }
        }
    }
    test_capsule_field_resolution_greatest! {test_aes_conflict_resolution, "aes", "AES", "aes", "{}", "number_of_blocks", "64", "128"}
    test_capsule_field_resolution_greatest! {test_flash_conflict_resolution, "flash", "FLASH", "flash", "{}", "buffer_size", "2048", "4096"}
    test_capsule_field_resolution_greatest! {test_hmac_conflict_resolution, "hmac", "HMAC", "hmac", "{}", "length", "32", "64"}

    macro_rules! test_capsule_reject_field_conflict {
        {$test:ident, $capsule:expr, $capsule_uc:expr, $peripheral:expr, $peripheral_type:expr, $field:expr, $field_value_1:expr, $field_value_2:expr} => {
            /// Check that the configurator rejects configurations with
            /// different field values for capsules that require all consumers
            /// to use the same value.
            #[test]
            #[should_panic]
            fn $test() {
                // This test must be configured such that $field_value_1 != $field_value_2.
                let capsule_config_1 = CapsuleConfig {
                    name: String::from($capsule),
                    attrs: HashMap::from([(String::from($field), String::from($field_value_1))]),
                };
                let capsule_config_2 = CapsuleConfig {
                    name: String::from($capsule),
                    attrs: HashMap::from([(String::from($field), String::from($field_value_2))]),
                };
                let mut opts = Opts {
                    mode: Mode::Cli,
                    chip: SupportedChip::EarlgreyCw310,
                    platform: String::from("AutogeneratedPlatform"),
                    stack_size: 2000,
                    scheduler: parse::SchedulerType::Cooperative,
                    syscall_filter: parse::SyscallFilterType::None,
                    processes: 0,
                    // Placeholder; we initialize the capsule configs directly.
                    capsules: vec![],
                    out: None,
                };
                // Check that the configurator panics on the value conflict.
                let _ = run_cli_inner(&mut opts, &[capsule_config_1, capsule_config_2]);
            }
        }
    }
    test_capsule_reject_field_conflict! {test_console_conflict, "console", "CONSOLE", "uart", "{}", "baud_rate", "123456", "123457"}
    test_capsule_reject_field_conflict! {test_lldb_conflict, "lldb", "LLDB", "uart", "{}", "baud_rate", "123456", "123457"}
}
