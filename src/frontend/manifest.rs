// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

use std::fs;
use std::path::PathBuf;
use std::error::Error;

use crate::parser;

pub fn prepare_manifest(config: &parser::Config) -> Result<(), Box<dyn Error>> {
    let manifest_path = config.base_path.join("AndroidManifest.xml");
    let cache_dir = config.base_path.join(".spawn").join("cache");
    let cache_manifest_path = cache_dir.join("AndroidManifest.xml");

    if !manifest_path.exists() {
        return Err(format!("AndroidManifest.xml not found {:?}", manifest_path).into());
    }

    let new_content = parser::generate_manifest(&manifest_path, config)?;

    fs::create_dir_all(&cache_dir)?;
    fs::write(&cache_manifest_path, new_content)?;

    Ok(())
}