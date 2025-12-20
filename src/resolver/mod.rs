// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

pub mod artifact;
pub mod local_cache;
pub mod pom;
mod unpacker;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::cmp::Ordering;

use colored::Colorize;
use quick_xml::{events::Event, Reader};
use rayon::prelude::*;
use reqwest::blocking::Client;

use crate::resolver::artifact::Artifact;
use crate::resolver::pom::{parse as parse_pom, process_imports, resolve_val, Pom};

pub struct Resolver {
    pub resolved_artifacts: HashMap<String, Artifact>,

    client: Client,
    repositories: Vec<String>,
    cache_root: PathBuf,
}

impl Resolver {
    pub fn new(user_repos: Vec<String>, cache_base: &Path) -> Self {
        let cache_root = cache_base.join("repository");
        fs::create_dir_all(&cache_root).ok();

        let mut repositories = vec![
            "https://dl.google.com/dl/android/maven2/".to_string(),
            "https://dl.google.com/android/maven2/".to_string(),
            "https://repo1.maven.org/maven2/".to_string(),

            // Зеркало для РФ и Китая, обходит блокировки от гугла
            "https://repo.huaweicloud.com/repository/maven/".to_string(),
        ];

        for r in user_repos {
            let mut url = r.trim().to_string();
            
            if !url.ends_with('/') {
                url.push('/');
            }

            if !repositories.contains(&url) {
                repositories.push(url);
            }
        }

        // На всякий слкчай клиент пытается косить под браузер
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::default())
            .hickory_dns(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            resolved_artifacts: HashMap::new(),
            client,
            repositories,
            cache_root,
        }
    }

    pub fn resolve(&mut self, root_coords: Vec<String>) {
        let mut queue: Vec<Artifact> = root_coords
            .iter()
            .filter_map(|c| Artifact::from_coords(c))
            .collect();

        let mut all_versions: HashMap<String, Vec<Artifact>> = HashMap::new();
        let mut visited = HashSet::new();

        println!("{} Resolving graph", "TASK:".green());

        while !queue.is_empty() {
            let mut next_queue = Vec::new();

            for art in queue {
                let id = art.id();
                let versions = all_versions.entry(id.clone()).or_insert_with(Vec::new);

                if versions.iter().any(|e| e.version == art.version) {
                    continue;
                }

                versions.push(art.clone());

                if let Ok(pom_path) = self.fetch_artifact(&art, "pom") {
                    if let Ok(xml) = fs::read_to_string(pom_path) {
                        let mut pom_data = self.get_effective_pom(&art, &xml);

                        for repo in pom_data.repositories.drain(..) {
                            let mut url = repo.trim().to_string();
                            
                            if !url.ends_with('/') {
                                url.push('/');
                            }

                            if !self.repositories.contains(&url) {
                                self.repositories.push(url);
                            }
                        }

                        for dep in &mut pom_data.dependencies {
                            dep.artifact.version = resolve_val(&dep.artifact.version, &pom_data.properties, &art);

                            if dep.artifact.version.is_empty() {
                                if let Some(man) = pom_data.dep_management.iter().find(|m| {
                                    m.artifact.group == dep.artifact.group && m.artifact.name == dep.artifact.name
                                }) {
                                    dep.artifact.version = resolve_val(&man.artifact.version, &pom_data.properties, &art);
                                }
                            }
                        }

                        println!(" Resolved {} ({} deps)", art, pom_data.dependencies.len());

                        for dep in pom_data.dependencies {
                            if dep.scope.as_ref().map_or(false, |s| s == "test" || s == "provided") {
                                continue;
                            }

                            let mut trans_art = dep.artifact;

                            if trans_art.version.is_empty() {
                                eprintln!("{} No version for dependency {}", "WARN:".yellow(), trans_art);
                                continue;
                            }

                            if trans_art.is_dynamic() {
                                match self.resolve_dynamic_version(&trans_art) {
                                    Ok(v) => trans_art.version = v,
                                    Err(e) => {
                                        eprintln!(
                                            "{} Failed to resolve dynamic version for {}: {}",
                                            "ERROR:".red(),
                                            trans_art,
                                            e
                                        );
                                        continue;
                                    }
                                }
                            }

                            let v_id = format!("{}:{}", trans_art.id(), trans_art.version);
                            if visited.insert(v_id) {
                                next_queue.push(trans_art);
                            }
                        }
                    }
                }
            }

            queue = next_queue;
        }

        self.resolve_version_conflicts(&all_versions);
    }

    fn get_effective_pom(&self, art: &Artifact, xml: &str) -> Pom {
        let mut pom = parse_pom(xml, art);

        process_imports(&mut pom.dep_management, self);

        if let Some(parent_art) = pom.parent.clone() {
            if let Ok(parent_path) = self.fetch_artifact(&parent_art, "pom") {
                if let Ok(parent_xml) = fs::read_to_string(&parent_path) {
                    let parent_pom = self.get_effective_pom(&parent_art, &parent_xml);

                    for (k, v) in parent_pom.properties {
                        pom.properties.entry(k).or_insert(v);
                    }

                    let mut merged = pom.dep_management;
                    merged.extend(parent_pom.dep_management);
                    pom.dep_management = merged;

                    pom.repositories.extend(parent_pom.repositories);
                }
            }
        }

        pom
    }

    fn resolve_version_conflicts(&mut self, all_versions: &HashMap<String, Vec<Artifact>>) {
        for (id, versions) in all_versions {
            if versions.len() > 1 {
                println!(" Conflict detected for {}: {} versions found", id, versions.len());
            }

            let winner = versions.iter().max_by(|a, b| {
                let cmp_result = version_compare::compare(&a.version, &b.version);
                match cmp_result {
                    Ok(cmp) => match cmp {
                        version_compare::Cmp::Gt | version_compare::Cmp::Ge => Ordering::Greater,
                        version_compare::Cmp::Eq => Ordering::Equal,
                        version_compare::Cmp::Lt | version_compare::Cmp::Le => Ordering::Less,
                        version_compare::Cmp::Ne => Ordering::Equal,
                    },

                    Err(_) => Ordering::Equal, // ошибка парсинга, можно считать равными
                }
            });

            if let Some(winner) = winner.cloned() {
                self.resolved_artifacts.insert(id.clone(), winner.clone());

                if versions.len() > 1 {
                    println!(" Selected version {} for {}", winner.version, id);
                }
            }
        }
    }

    pub fn download_all(&self) {
        println!("{} Download dependencies", "TASK:".green());

        self.resolved_artifacts.par_iter().for_each(|(_, art)| {
            let mut downloaded: Option<PathBuf> = None;

            if let Ok(path) = self.fetch_artifact(art, "aar") {
                println!(" Aar: {}", art.name);
                downloaded = Some(path);

                if let Err(e) = crate::resolver::unpacker::unpack_aar(downloaded.as_ref().unwrap()) {
                    eprintln!("{} unpack error {}: {}", "ERROR:".red(), art.name, e);
                }
            } else if let Ok(path) = self.fetch_artifact(art, "jar") {
                println!(" Jar: {}", art.name);
                downloaded = Some(path);
            }

            if downloaded.is_none() {
                eprintln!("{} {}", "ERROR:".red(), art);
            }
        });
    }

    fn fetch_artifact(&self, art: &Artifact, ext: &str) -> Result<PathBuf, String> {
        let mut rel_path = art.get_path(ext);
        let mut file_name = format!("{}-{}.{}", art.name, art.version, ext);

        if art.is_snapshot() {
            let snapshot_version = self.resolve_snapshot(art, ext)?;
            file_name = format!("{}-{}.{}", art.name, snapshot_version, ext);

            let g = art.group.replace('.', "/");
            rel_path = format!("{}/{}/{}/{}", g, art.name, art.version, file_name);
        }

        let full_path = self.cache_root.join(&rel_path);

        if full_path.exists() {
            return Ok(full_path);
        }

        for repo in &self.repositories {
            let url = format!("{}{}", repo, rel_path);

            if let Ok(resp) = self.client.get(&url).send() {
                if resp.status().is_success() {
                    fs::create_dir_all(full_path.parent().unwrap()).ok();

                    let bytes = resp.bytes().map_err(|e| e.to_string())?;
                    let mut out = fs::File::create(&full_path).map_err(|e| e.to_string())?;
                    out.write_all(&bytes).map_err(|e| e.to_string())?;

                    return Ok(full_path);
                }
            }
        }

        Err(format!("{} Not found: {}", "ERROR".red(), rel_path))
    }

    fn fetch_metadata(&self, art: &Artifact, per_version: bool) -> Result<String, String> {
        let rel_path = art.get_metadata_path(per_version);
        let full_path = self.cache_root.join(&rel_path);

        // Проверка кэша
        if full_path.exists() {
            if let Ok(metadata) = fs::metadata(&full_path) {
                // maven-metadata.xml обычно > 256 байт
                if metadata.len() >= 256 {
                    if let Ok(content) = fs::read_to_string(&full_path) {
                        if !content.trim().is_empty() {
                            return Ok(content);
                        }
                    }
                }
            }

            // Файл битый или пустой, желательно удалить
            eprintln!("WARN: Removing corrupted metadata cache: {}", full_path.display());
            let _ = fs::remove_file(&full_path);
        }

        // Скачивание из репозиториев
        for repo in &self.repositories {
            let url = format!("{}{}", repo, rel_path);

            match self.client.get(&url).send() {
                Ok(resp) if resp.status().is_success() => {
                    match resp.text() {
                        Ok(text) => {
                            if text.trim().is_empty() {
                                // Если ответ пустой, то пробуем следующий репозиторий
                                continue;
                            }

                            // Сохранение в кэш
                            if let Some(parent) = full_path.parent() {
                                let _ = fs::create_dir_all(parent);
                            }

                            if let Ok(mut file) = fs::File::create(&full_path) {
                                let _ = file.write_all(text.as_bytes());
                            }

                            return Ok(text);
                        }

                        Err(e) => return Err(format!("Failed to read response text: {}", e)),
                    }
                }
                Ok(resp) => {
                    // Если 200 просто идём дальше
                    continue;
                }
                Err(e) => {
                    // Сетевая ошибка, можно логировать, но тут идёт попытка использовать другой
                    // репозиторий для скачивания (continue переходит к другому репо)
                    eprintln!("{} Network error fetching metadata {}: {}", "WARN:".yellow(), url, e);
                    continue;
                }
            }
        }

        Err(format!("Metadata not found for {} at path {}", art, rel_path))
    }

    fn resolve_snapshot(&self, art: &Artifact, ext: &str) -> Result<String, String> {
        let xml = self.fetch_metadata(art, true)?;

        let mut reader = Reader::from_str(&xml);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut path = Vec::new();
        let mut cur_ext = String::new();
        let mut cur_val = String::new();
        let mut latest_val = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => path.push(String::from_utf8_lossy(e.local_name().as_ref()).into_owned()),
                
                Ok(Event::Text(e)) => {
                    let val = e.unescape().unwrap_or_default().into_owned();
                    let full_path = path.join("/");
                    
                    if full_path == "metadata/versioning/snapshotVersions/snapshotVersion/extension" {
                        cur_ext = val;
                    } else if full_path == "metadata/versioning/snapshotVersions/snapshotVersion/value" {
                        cur_val = val;
                    }
                }

                Ok(Event::End(_)) => {
                    let full_path = path.join("/");
                    if full_path == "metadata/versioning/snapshotVersions/snapshotVersion" {
                        if cur_ext == ext {
                            latest_val = cur_val.clone();
                        }

                        cur_ext.clear();
                        cur_val.clear();
                    }

                    path.pop();
                }

                Ok(Event::Eof) => break,
                
                _ => (),
            }

            buf.clear();
        }

        if latest_val.is_empty() {
            Err(format!("No snapshot version for ext {} in {}", ext, art))
        } else {
            Ok(latest_val)
        }
    }

    fn resolve_dynamic_version(&self, art: &Artifact) -> Result<String, String> {
        let xml = self.fetch_metadata(art, false)?;

        let mut reader = Reader::from_str(&xml);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut path = Vec::new();
        let mut latest = String::new();
        let mut release = String::new();
        let mut versions: Vec<String> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => path.push(String::from_utf8_lossy(e.local_name().as_ref()).into_owned()),
                
                Ok(Event::Text(e)) => {
                    let val = e.unescape().unwrap_or_default().into_owned();
                    let full_path = path.join("/");
                    
                    match full_path.as_str() {
                        "metadata/versioning/latest" => latest = val,
                        "metadata/versioning/release" => release = val,
                        "metadata/versioning/versions/version" => versions.push(val),
                        _ => {}
                    }
                }
                
                Ok(Event::End(_)) => {
                    path.pop();
                }
                
                Ok(Event::Eof) => break,
                
                _ => (),
            }

            buf.clear();
        }

        match art.version.as_str() {
            "LATEST" => if !latest.is_empty() {
                Ok(latest)
            } else {
                Err("No latest version".to_string())
            },

            "RELEASE" => if !release.is_empty() {
                Ok(release)
            } else {
                Err("No release version".to_string())
            },

            _ => {
                let range = &art.version;
                let mut matching: Vec<String> = versions
                    .into_iter()
                    .filter(|v| matches_range(v, range))
                    .collect();
                
                matching.sort_by(|a, b| {
                    let cmp_result = version_compare::compare(a, b);
                    match cmp_result {
                        Ok(cmp) => match cmp {
                            version_compare::Cmp::Gt | version_compare::Cmp::Ge => Ordering::Greater,
                            version_compare::Cmp::Eq => Ordering::Equal,
                            version_compare::Cmp::Lt | version_compare::Cmp::Le => Ordering::Less,
                            version_compare::Cmp::Ne => Ordering::Equal,
                        },
                        Err(_) => Ordering::Equal,
                    }
                });

                matching
                    .last()
                    .cloned()
                    .ok_or_else(|| "No matching version for range".to_string())
            }
        }
    }

    pub fn verify_all_artifacts_exist(&self) -> bool {
        for (_, artifact) in &self.resolved_artifacts {
            let aar_path = self.cache_root.join(artifact.get_path("aar"));
            let jar_path = self.cache_root.join(artifact.get_path("jar"));

            let aar_ok = aar_path.exists() && fs::metadata(&aar_path)
                .map(|m| m.len() >= 1024)
                .unwrap_or(false);

            let jar_ok = jar_path.exists() && fs::metadata(&jar_path)
                .map(|m| m.len() >= 1024)
                .unwrap_or(false);

            if !aar_ok && !jar_ok {
                eprintln!("{} missing {}", "WARN:".yellow(), artifact);
                return false;
            }
        }
        
        true
    }
}

fn matches_range(version: &str, range_str: &str) -> bool {
    let trimmed = range_str.trim();

    if !trimmed.contains(',') && !trimmed.contains('(') && !trimmed.contains('[') {
        return version_compare::compare(version, trimmed)
            .map(|c| c == version_compare::Cmp::Eq)
            .unwrap_or(false);
    }

    let (lower, upper) = if trimmed.contains(',') {
        let parts: Vec<&str> = trimmed.splitn(2, ',').collect();
        (parts[0], parts.get(1).copied().unwrap_or(""))
    } else {
        (trimmed, "")
    };

    let lower_incl = lower.starts_with('[');
    let lower_bound = lower.trim_start_matches(|c| c == '(' || c == '[');

    let upper_incl = upper.ends_with(']');
    let upper_bound = upper.trim_end_matches(|c| c == ')' || c == ']');

    let mut ok = true;

    if !lower_bound.is_empty() {
        if let Ok(cmp) = version_compare::compare(version, lower_bound) {
            ok &= match cmp {
                version_compare::Cmp::Gt | version_compare::Cmp::Ge => !lower_incl,
                version_compare::Cmp::Eq => lower_incl,
                _ => false,
            };
        } else {
            ok = false;
        }
    }

    if !upper_bound.is_empty() {
        if let Ok(cmp) = version_compare::compare(version, upper_bound) {
            ok &= match cmp {
                version_compare::Cmp::Lt | version_compare::Cmp::Le => !upper_incl,
                version_compare::Cmp::Eq => upper_incl,
                _ => false,
            };
        } else {
            ok = false;
        }
    }

    ok
}