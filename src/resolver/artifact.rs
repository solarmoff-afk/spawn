// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Artifact {
    pub group: String,
    pub name: String,
    pub version: String,
}

impl Artifact {
    pub fn new(group: &str, name: &str, version: &str) -> Self {
        // В maven в названии версии бывают квадратные скобки что мешает
        // разрешить зависимость, поэтому нужно их убрать. Например [1.6.1]
        // превращается в 1.6.1
        let clean_version = version
            .trim_matches(|c| c == '[' || c == ']')
            .to_string();

        Self {
            group: group.trim().to_string(),
            name: name.trim().to_string(),
            version: clean_version,
        }
    }

    pub fn from_coords(coords: &str) -> Option<Self> {
        let parts: Vec<&str> = coords.split(':').collect();
        if parts.len() < 3 {
            return None;
        }
        
        Some(Self::new(parts[0], parts[1], parts[2]))
    }

    pub fn id(&self) -> String {
        format!("{}:{}", self.group, self.name)
    }

    pub fn get_path(&self, ext: &str) -> String {
        let g = self.group.replace('.', "/");
        format!("{}/{}/{}/{}-{}.{}", g, self.name, self.version, self.name, self.version, ext)
    }
}

impl std::fmt::Display for Artifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.group, self.name, self.version)
    }
}