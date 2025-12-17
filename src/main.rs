// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

mod build_system;

use build_system::{BuildSystem, Actions};

fn main() {
    let mut build_system = BuildSystem::new();
    let action = build_system.get_action();

    match action {
        Actions::build_apk => {

        },

        Actions::help => {
            build_system.print_help();
        },

        _ => todo!(),
    }
}
