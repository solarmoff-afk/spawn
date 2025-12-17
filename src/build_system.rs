// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

use std::env;

use colored::Colorize;

/// Это перечисление нужно для удобного распознавания действия которое передаётся
/// вторым аргументом (индекс 1) при запуске spawn и означает "Что именно сделать?"
/// Если там ничего нет либо такого варианта нет в match, то это help, то есть
/// вывести справку по использованию spawn
pub enum Actions {
    help = 0,
    build_apk = 1,
    build_aab = 2,
    up = 3,
    clean = 4,
}

/// Структура которая используется для глобального хранения аргументов системы
/// сборки и другой общей информации
pub struct BuildSystem {
    pub args: Vec<String>,
}

impl BuildSystem {
    pub fn new() -> Self {
        Self {
            args: env::args().collect(),
        }
    }

    /// Эта функция берёт аргумент действия из args (Второй аргумент, первый индекс)
    /// и возвращает либо help из Actions (0) либо конкретный вариант. Используется
    /// enum для удобства обработки результата
    pub fn get_action(&mut self) -> Actions {
        if self.args.len() >= 2 {
            return match self.args[1].as_str() {
                "apk"   => Actions::build_apk,
                "aab"   => Actions::build_aab,
                "up"    => Actions::up,
                "clean" => Actions::clean,
                _       => Actions::help,
            }
        }

        // Если аргументов меньше 2 то возвращаем help
        return Actions::help;
    }

    /// [WAIT DOC]
    pub fn print_help(&mut self) {
        println!("{} is easy-to-use build system for building android apps", "Spawn".green().bold());
        println!("  - spawn apk my.toml     | build {} file", "apk".red());
        println!("  - spawn aab my.toml     | build {} file (for Google Play)", "aab".red());
        println!("  - spawn up sdk/ndk      | download/update android sdk/ndk");
        println!("  - spawn clean           | delete all cache");
        println!("  - spawn help            | show help info");
        println!("If you want use multiconfig mode, use");
        println!("  - spawn {} my.toml, my2.toml, my3.toml", "apk".red());
    }
}