// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

mod build_system;
mod parser;
mod frontend;
mod resolver;

use build_system::{BuildSystem, Actions};

fn main() {
    let mut build_system = BuildSystem::new();
    let action = build_system.get_action();

    match action {
        Actions::build_apk => {
            let paths = get_paths(build_system.args.clone());
            if paths.is_empty() {
                build_system.print_help();
                panic!("No toml file provided");
            }

            let config = frontend::prepare(paths)
                .expect("Failed to prepare project");

            println!("Build finish");
        },

        Actions::help => {
            build_system.print_help();
        },

        _ => todo!(),
    }
}

fn get_paths(args: Vec<String>) -> Vec<String> {
    if args.len() >= 3 {
        return args[2..].to_vec();
    }

    vec![]
}