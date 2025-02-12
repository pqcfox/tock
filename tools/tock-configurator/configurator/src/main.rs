// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use clap::Parser;
use tock_configurator::{init, run_cli_mode, Mode, Opts};

fn main() {
    let opts = Opts::parse();
    match opts.mode {
        Mode::Gui => {
            let mut configurator = init();
            configurator.run()
        }
        Mode::Cli => run_cli_mode(opts),
    };
}
