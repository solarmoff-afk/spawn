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
