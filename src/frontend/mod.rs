// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

pub mod manifest;

use std::fs;
use std::path::PathBuf;
use sha2::{Sha256, Digest};
use colored::Colorize;

use crate::parser;

/// Фронтенд функция для выполнения нулнвого этапа (Работа с зависимостями и конфигами)
pub fn prepare(paths: Vec<String>) -> Result<parser::Config, Box<dyn std::error::Error>> {
    let config = parser::load(paths)?;
    manifest::prepare_manifest(&config)?;

    if let Some(deps) = &config.dependencies {
        let cache_dir = config.base_path.join(".spawn").join("cache");
        let lock_file = cache_dir.join("resolve.lock");
        
        // нужно создать отпечаток зависимостей чтобы сохранить его в .lock файл 
        let current_fingerprint = generate_fingerprint(&config);

        // Эта переменная нужна чтобы понять разрешать ли зависимости
        let mut need_resolve = true;

        if lock_file.exists() {
            let saved_fingerprint = fs::read_to_string(&lock_file)?;
            
            // Если текущий отпечаток зависимостей совпадает с отпечатком из
            // .lock файла то ничего не изменилось, разрешать ничего не нужно
            if saved_fingerprint == current_fingerprint {
                need_resolve = false;
            }
        }

        if need_resolve {
            let home = dirs::home_dir().expect("Cannot find home dir");
            let mut resolver = crate::resolver::Resolver::new(vec![], &home.join(".spawn"));

            let root_deps: Vec<String> = deps.iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect();

            resolver.resolve(root_deps);
            resolver.download_all();
            
            let all_downloaded = resolver.verify_all_artifacts_exist();

            if all_downloaded {
                fs::create_dir_all(&cache_dir)?;
                fs::write(&lock_file, current_fingerprint)?;
            }

            warn!("Some dependencies failed to download. Cache lock not written");
            info!("Run the build again or check your network connection");
        } else {
            info!("{} Dependencies are up-to-date", "CACHED:".green());
        }
    }

    Ok(config)
}

/// Эта функция получает конфиг и генерирует хэш зависимостей
fn generate_fingerprint(config: &parser::Config) -> String {
    let mut hasher = Sha256::new();
    
    if let Some(deps) = &config.dependencies {
        // Сортировка нужна для стабильности хэша
        let mut sorted_deps: Vec<_> = deps.iter().collect();
        sorted_deps.sort_by_key(|a| a.0);
        
        for (k, v) in sorted_deps {
            hasher.update(k);
            hasher.update(v);
        }
    }

    // Хэширование репозиториев
    if let Some(repos) = &config.repositories {
        for r in repos {
            hasher.update(r);
        }
    }

    format!("{:x}", hasher.finalize())
}