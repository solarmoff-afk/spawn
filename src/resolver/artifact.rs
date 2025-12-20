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
        // В мавене иногда бывает *, походу это никак не используется поэтому
        // можно просто вывести варн и вернуть артефакт с именем INVALID
        // потому-что потому
        if name == "*" {
            eprintln!("WARN: Skipping invalid artifact with name '*'");
            
            Self {
                group: group.trim().to_string(),
                name: "INVALID".to_string(),
                version: version.to_string(),
            }
        } else {
            // В maven в названии версии бывают квадратные скобки что мешает
            // разрешить зависимость, поэтому нужно их убрать. Например [1.6.1]
            // превращается в 1.6.1
            let clean_version = version.trim_matches(|c| c == '[' || c == ']').to_string();

            Self {
                group: group.trim().to_string(),
                name: name.trim().to_string(),
                version: clean_version,
            }
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
        format!(
            "{}/{}/{}/{}-{}.{}",
            g, self.name, self.version, self.name, self.version, ext
        )
    }

    pub fn is_snapshot(&self) -> bool {
        self.version.ends_with("-SNAPSHOT")
    }

    pub fn is_dynamic(&self) -> bool {
        let v = &self.version;
        v == "LATEST"
            || v == "RELEASE"
            || v.contains('[')
            || v.contains('(')
            || v.contains(',')
    }

    pub fn get_metadata_path(&self, per_version: bool) -> String {
        let g = self.group.replace('.', "/");
        if per_version {
            format!("{}/{}/{}/maven-metadata.xml", g, self.name, self.version)
        } else {
            format!("{}/{}/maven-metadata.xml", g, self.name)
        }
    }
}

impl std::fmt::Display for Artifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.group, self.name, self.version)
    }
}