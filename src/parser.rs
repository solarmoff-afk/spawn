// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

use serde::Deserialize;
use toml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Функция для парсинга toml конфига. Spawn поддерживает несколько путей, поэтому
/// передаётся не путь к 1 файлу, а вектор из путей к файлам. Возвращает итоговое
/// представление конфига
pub fn load<P: AsRef<Path>>(paths: Vec<P>) -> Result<ProjectConfig, String> {
    if paths.is_empty() {
        return Ok(ProjectConfig::default());
    }

    let mut merged_value = Value::Table(toml::map::Map::new());

    // Чтение каждого файла в векторе путей, превращение в toml представление и слияние
    for path in paths {
        let path_ref = path.as_ref();
        let content = fs::read_to_string(path_ref)
            .map_err(|e| format!("File read error {:?}: {}", path_ref, e))?;

        let value: Value = toml::from_str(&content)
            .map_err(|e| format!("Toml syntax error {:?}: {}", path_ref, e))?;

        // Сливаем конфиг файл с друними
        merge_toml_values(&mut merged_value, value);
    }

    let config: ProjectConfig = merged_value.try_into()
        .map_err(|e| format!("Config structure error: {}", e))?;

    Ok(config)
}

/// Эта функция нужна чтобы слить все toml конфиг файлы проекта в 1 представление
fn merge_toml_values(base: &mut Value, append: Value) {
    match (base, append) {
        (Value::Table(base_map), Value::Table(append_map)) => {
            for (k, v) in append_map {
                let base_entry = base_map.entry(k).or_insert(Value::Table(toml::map::Map::new()));
                merge_toml_values(base_entry, v);
            }
        }
        (base_val, append_val) => *base_val = append_val,
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct ProjectConfig {
    // Основная информация про приложение
    pub package: Option<Package>,
    
    // Минимальный и целевой sdk
    pub sdk: Option<Sdk>,
    
    // Настройки подписи
    pub sign: Option<SignConfig>,

    // Разрешения приложения
    #[serde(default)]
    pub manifest: Manifest,

    // Команды при старте сборки
    #[serde(default)]
    pub prepare: HashMap<String, String>,

    // Наборы команд которые будут выполняться для каждого указанного в них
    //  этапа сборки
    #[serde(default)]
    pub task: Tasks,

    #[serde(default)]
    pub options: Options,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Package {
    pub package: Option<String>,
    pub version: Option<String>,     // android:versionName
    pub version_code: Option<u32>,   // android:versionCode
    pub label: Option<String>,       // android:label
    pub icon: Option<String>,        // android:icon
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Sdk {
    pub target_sdk: Option<String>,
    pub min_sdk: Option<String>,
    pub compile_sdk: Option<String>, 
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    #[serde(default)]
    pub permission: Vec<String>,
    
    #[serde(default)]
    pub manage_external_storage: bool, 

    #[serde(default)]
    pub legacy_storage: bool,

    #[serde(default)]
    pub extract_native_libs: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SignConfig {
    pub debug: Option<SignProfile>,
    pub release: Option<SignProfile>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct SignProfile {
    pub store_file: Option<String>,
    pub key_alias: Option<String>,
    pub store_password: Option<String>,
    pub key_password: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Tasks {
    pub resource: Option<HashMap<String, String>>,
    pub java: Option<HashMap<String, String>>,
    pub dex: Option<HashMap<String, String>>,
    pub align: Option<HashMap<String, String>>,
    pub sign: Option<HashMap<String, String>>,
    pub clean: Option<HashMap<String, String>>,
    pub finish: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Options {
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub cache: bool,
}