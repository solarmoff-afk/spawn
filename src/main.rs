// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

mod build_system;
mod parser;

use build_system::{BuildSystem, Actions};

fn main() {
    let mut build_system = BuildSystem::new();
    let action = build_system.get_action();

    match action {
        Actions::build_apk => {
            let config = parser::load(get_paths(build_system.args))
                .expect("Failed to create config from toml files");

            build_system.set_config(config);
            
            // UNSAFE TESTING
            println!("Package.package: {}", config.package.unwrap().package.unwrap());
        },

        Actions::help => {
            build_system.print_help();
        },

        _ => todo!(),
    }
}

/// [WAIT DOC]
fn get_paths(args: Vec<String>) -> Vec<String> {
    let len = args.len();

    if len == 2 { return args[2..2].to_vec(); }
    else if len > 2 { return args[2..len].to_vec(); }

    return vec![]
}