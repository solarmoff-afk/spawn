// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

/// Макрос для вывода информации о текущей задаче/этапе сборки
/// Выводит "TASK: Сообщение" где TASK зелёным и жирным
#[macro_export]
macro_rules! task {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        println!("{} {}", "TASK:".green().bold(), format!($($arg)*));
    }};
}

/// Макрос для вывода обычной информации без префиксов
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        println!("{}", format!($($arg)*));
    }};
}

/// Макрос для вывода важных заметок. Префикс "NOTE:", цвет синий и шрифт жирный
#[macro_export]
macro_rules! note {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        println!("{} {}", "NOTE:".blue().bold(), format!($($arg)*));
    }};
}

/// Вывод сообщений о старте хуков. Префикс HOOK, цвет фиолетовый, шрифт жирный
#[macro_export]
macro_rules! hook {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        println!("{} {}", "HOOK:".purple().bold(), format!($($arg)*));
    }};
}

/// Макрос для вывода предупреждений. Выводит в stderr "WARN: Сообщение" где
/// префикс WARN: жёлтым цветом и жирным шрифтом
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        eprintln!("{} {}", "WARN:".yellow().bold(), format!($($arg)*));
    }};
}

/// Макрос для ошибок. Выводит в stderr "ERROR: Сообщение". Цвет префикса красный,
/// шрифт жирный. Используется чтобы сообщить что что-то пошло не так
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        eprintln!("{} {}", "ERROR:".red().bold(), format!($($arg)*));
    }};
}

/// Макрос для фатальных ошибок. Выводит в stderr "FATAL: Сообщение". Цвет префикса красный,
/// шрифт жирный. Используется чтобы сообщить фатальную ошибку при которой продолжение
/// сборки невозможно и закрыть программу
#[macro_export]
macro_rules! fatal {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        eprintln!("{} {}", "FATAL:".red().bold().on_black(), format!($($arg)*));
        std::process::exit(1);
    }};
}