// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

pub mod manifest;
use std::error::Error;

use crate::parser;

/// Фронтенд функция для выполнения нулнвого этапа (Работа с зависимостями и конфигами)
pub fn prepare(paths: Vec<String>) -> Result<parser::Config, Box<dyn Error>> {
    let config = parser::load(paths)?;

    manifest::prepare_manifest(&config)?;

    Ok(config)
}