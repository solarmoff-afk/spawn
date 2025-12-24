// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

pub mod manifest;
pub mod ninja_generator;

use std::fs;
use sha2::{Sha256, Digest};
use colored::Colorize;
use dirs::home_dir;

use crate::parser;
use crate::resolver::Resolver;

/// Подготавливает проект и возвращает конфиг + resolver (если были зависимости)
pub fn prepare(paths: Vec<String>) -> Result<(parser::Config, Option<Resolver>), Box<dyn std::error::Error>> {
    let config = parser::load(paths)?;
    manifest::prepare_manifest(&config)?;

    let mut resolver = None;

    if let Some(deps) = &config.dependencies {
        if !deps.is_empty() {
            let project_cache_dir = config.base_path.join(".spawn").join("cache");
            let lock_file = project_cache_dir.join("resolve.lock");

            let current_fingerprint = generate_fingerprint(&config);

            let mut need_resolve = true;
            if lock_file.exists() {
                if let Ok(saved) = fs::read_to_string(&lock_file) {
                    if saved.trim() == current_fingerprint {
                        need_resolve = false;
                        info!("{} Dependencies are up-to-date", "CACHED:".green());
                    }
                }
            }

            if need_resolve {
                task!("Resolving dependencies");

                let home = home_dir().expect("Cannot find home directory");
                let global_repository = home.join(".spawn").join("repository");

                let mut r = Resolver::new(
                    config.repositories.clone().unwrap_or_default(),
                    &global_repository,
                );

                let root_deps: Vec<String> = deps.iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect();

                r.resolve(root_deps);
                r.download_all();

                let all_downloaded = r.verify_all_artifacts_exist();

                if all_downloaded {
                    fs::create_dir_all(&project_cache_dir)?;
                    fs::write(&lock_file, current_fingerprint)?;
                    info!("Dependencies resolved and cached");
                } else {
                    warn!("Some dependencies failed to download will retry next run");
                }

                resolver = Some(r);
            }
        }
    } else {
        info!("No dependencies section — building without external libs");
    }

    Ok((config, resolver))
}

fn generate_fingerprint(config: &parser::Config) -> String {
    let mut hasher = Sha256::new();

    if let Some(deps) = &config.dependencies {
        let mut sorted: Vec<_> = deps.iter().collect();
        sorted.sort_by_key(|a| a.0);

        for (k, v) in sorted {
            hasher.update(k.as_bytes());
            hasher.update(b":");
            hasher.update(v.as_bytes());
        }
    }

    if let Some(repos) = &config.repositories {
        for r in repos {
            hasher.update(r.as_bytes());
        }
    }

    format!("{:x}", hasher.finalize())
}