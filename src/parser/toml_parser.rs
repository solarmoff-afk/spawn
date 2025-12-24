// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

use serde::Deserialize;
use walkdir::WalkDir;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub package: Option<PackageInfo>,
    pub sign: Option<SignInfo>,
    pub repositories: Option<Vec<String>>,
    pub dependencies: Option<HashMap<String, String>>,
    
    #[serde(skip)]
    pub base_path: PathBuf,

    #[serde(skip)]
    pub modules: Vec<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PackageInfo {
    pub package: Option<String>,
    pub version: Option<String>,
    pub version_code: Option<u32>,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SignInfo {
    pub keystore: String,
    pub alias: String,
}

/// [WAIT DOC]
pub fn load_configs(paths: Vec<String>) -> Result<Config, Box<dyn std::error::Error>> {
    let first_toml = PathBuf::from(&paths[0]);
    
    let base_dir = first_toml.parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let mut target_paths = Vec::new();

    if paths.len() == 1 {
        target_paths.push(first_toml.clone());

        // Нужно пройтись по всем подпапкам и если там есть module.toml, то
        // это модуль и нужно использлвать зависимости из него
        for entry in WalkDir::new(&base_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_name() == "module.toml" && entry.path() != first_toml {
                target_paths.push(entry.path().to_path_buf());
            }
        }
    } else {
        for p in paths {
            target_paths.push(PathBuf::from(p));
        }
    }

    let mut final_config = Config {
        base_path: base_dir,
        modules: target_paths.clone(),
        ..Default::default()
    };

    let mut all_deps = HashMap::new();
    let mut all_repos = Vec::new();

    for (index, path) in target_paths.iter().enumerate() {
        let content = fs::read_to_string(path)?;
        let parsed: Config = toml::from_str(&content)?;

        if index == 0 {
            final_config.package = parsed.package;
            final_config.sign = parsed.sign;
        }

        if let Some(deps) = parsed.dependencies {
            all_deps.extend(deps);
        }
        
        if let Some(repos) = parsed.repositories {
            all_repos.extend(repos);
        }
    }
    
    // Зависимости
    final_config.dependencies = Some(all_deps);

    // Репозитории (откуда скачивать зависимости)
    final_config.repositories = Some(all_repos);
    
    Ok(final_config)
}