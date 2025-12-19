// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

use std::path::{Path, PathBuf};
use std::fs;

use crate::resolver::artifact::Artifact;

pub struct LocalCache {
    pub root: PathBuf,
}

impl LocalCache {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not find home directory");
        
        // Все зависимости выгружаются в HOME/.spawn/repository, это
        // глобальный кэш для всех spawn проектов
        let root = home.join(".spawn").join("repository");
        
        fs::create_dir_all(&root).ok();
        
        Self {
            root
        }
    }

    pub fn get_artifact_path(&self, artifact: &Artifact, ext: &str) -> PathBuf {
        self.root.join(artifact.to_maven_path(ext))
    }

    pub fn exists(&self, artifact: &Artifact, ext: &str) -> bool {
        self.get_artifact_path(artifact, ext).exists()
    }
}