// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

pub mod toml_parser;
pub mod manifest_generator;

pub use toml_parser::Config;
use std::error::Error;
use std::path::Path;

pub fn load(paths: Vec<String>) -> Result<Config, Box<dyn Error>> {
    toml_parser::load_configs(paths)
}

pub fn generate_manifest(path: &Path, config: &Config) -> Result<String, Box<dyn Error>> {
    manifest_generator::generate_manifest(path, config)
}