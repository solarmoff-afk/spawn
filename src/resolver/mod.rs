// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

pub mod artifact;
pub mod pom;
mod unpacker;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use rayon::prelude::*;
use reqwest::blocking::Client;
use colored::Colorize;

use crate::resolver::artifact::Artifact;

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

        // Зеркало хуавей на случай если пользователь находится в Китае, России
        // лиюо другой стране под санкциями
        let mut repositories = vec![
            "https://dl.google.com/dl/android/maven2/".to_string(), 
            "https://dl.google.com/android/maven2/".to_string(),    
            "https://repo1.maven.org/maven2/".to_string(),          
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
        let mut queue: Vec<Artifact> = root_coords.iter()
            .filter_map(|c| {
                let parsed = Artifact::from_coords(c);
                parsed
            })
            .collect();
        let mut visited = HashSet::new();

        println!("{} Resolving graph", "TASK:".green());

        while !queue.is_empty() {
            let mut next_queue = Vec::new();
            for art in queue {
                let id = art.id();
                let mut proceed = false;

                if let Some(existing) = self.resolved_artifacts.get(&id) {
                    // семантическое сравнение версий
                    let cmp = version_compare::compare(&art.version, &existing.version)
                        .unwrap_or(version_compare::Cmp::Eq);
                    
                    match cmp {
                        version_compare::Cmp::Gt => {
                            proceed = true;
                        }
                        
                        version_compare::Cmp::Lt => {
                            proceed = false;
                        }
                        
                        version_compare::Cmp::Eq => {
                            proceed = false;
                        }

                        _ => {
                            proceed = true;
                        }
                    }
                } else {
                    proceed = true;
                }
                
                if proceed {
                    self.resolved_artifacts.insert(id.clone(), art.clone());

                    let fetch_result = self.fetch_artifact(&art, "pom");
                    if let Ok(pom_path) = fetch_result {
                        let read_result = fs::read_to_string(pom_path);
                        if let Ok(xml) = read_result {
                            let pom_data = pom::parse(&xml, &art);
                            println!(" Resolved {} ({} deps)", art, pom_data.dependencies.len());
                            
                            for dep in pom_data.dependencies {
                                let trans_art = dep.artifact;
                                let v_id = format!("{}:{}", trans_art.id(), trans_art.version);
                                
                                if !visited.contains(&v_id) {
                                    visited.insert(v_id);
                                    next_queue.push(trans_art);
                                }
                            }
                        }
                    }
                }
            }

            queue = next_queue;
        }
    }

    pub fn download_all(&self) {
        println!("{} Download dependencies", "TASK:".green());

        self.resolved_artifacts.par_iter().for_each(|(_, art)| {
            let mut downloaded_path = None;

            if let Ok(path) = self.fetch_artifact(art, "aar") {
                println!(" Aar: {}", art.name);
                downloaded_path = Some(path);
                
                // Aar это просто zip архив как и jar, но не суть, главное что в этом
                // архиве лежит jar, res и так далее, а это нужно поэтому распаковываем aar
                if let Err(e) = crate::resolver::unpacker::unpack_aar(&downloaded_path.as_ref().unwrap()) {
                    eprintln!("{} unpack error {}: {}", "ERROR:".red(), art.name, e);
                }
            } else if let Ok(path) = self.fetch_artifact(art, "jar") {
                println!(" Jar: {}", art.name);
                downloaded_path = Some(path);
            }

            if downloaded_path.is_none() {
                eprintln!("{} {}", "ERROR:".red(), art);
            }
        });
    }

    fn fetch_artifact(&self, art: &Artifact, ext: &str) -> Result<PathBuf, String> {
        let rel_path = art.get_path(ext);
        let full_path = self.cache_root.join(&rel_path);

        if full_path.exists() {
            return Ok(full_path);
        }

        for repo in &self.repositories {
            let url = format!("{}{}", repo, rel_path);

            if let Ok(resp) = self.client.get(&url).send() {
                if resp.status().is_success() {
                    fs::create_dir_all(full_path.parent().unwrap()).ok();
                    
                    let mut out = fs::File::create(&full_path).map_err(|e| e.to_string())?;
                    let mut content = resp;
                    std::io::copy(&mut content, &mut out).map_err(|e| e.to_string())?;
                    
                    return Ok(full_path);
                }
            }
        }

        Err(format!("{} Not found: {}", "ERROR".red(), rel_path))
    }
}
