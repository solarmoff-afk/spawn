// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

use quick_xml::{events::Event, Reader};
use std::collections::HashMap;
use std::fs;

use crate::resolver::artifact::Artifact;
use crate::resolver::Resolver;

#[derive(Debug, Default)]
pub struct Pom {
    pub properties: HashMap<String, String>,
    pub dependencies: Vec<DependencyEntry>,
    pub dep_management: Vec<DependencyEntry>,
    pub repositories: Vec<String>,
    pub parent: Option<Artifact>,
}

#[derive(Debug, Clone)]
pub struct DependencyEntry {
    pub artifact: Artifact,
    pub scope: Option<String>,
    pub entry_type: Option<String>,
}

pub fn parse(xml: &str, _current_art: &Artifact) -> Pom {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut pom = Pom::default();
    let mut path = Vec::new();

    let mut cur_g = String::new();
    let mut cur_a = String::new();
    let mut cur_v = String::new();
    let mut cur_s = String::new();
    let mut cur_t = String::new();

    let mut cur_p_g = String::new();
    let mut cur_p_a = String::new();
    let mut cur_p_v = String::new();

    let mut cur_repo_url = String::new();
    let mut section = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                path.push(name.clone());

                let full = path.join("/");
                if full == "project/dependencyManagement/dependencies" {
                    section = "dep_management".to_string();
                } else if full == "project/dependencies" {
                    section = "dependencies".to_string();
                }
            }

            Ok(Event::Text(e)) => {
                let val = e.unescape().unwrap_or_default().into_owned();
                let full_path = path.join("/");

                if full_path.starts_with("project/properties/") {
                    let key = full_path
                        .strip_prefix("project/properties/")
                        .unwrap()
                        .to_string();
                    pom.properties.insert(key, val.clone());
                }

                match full_path.as_str() {
                    "project/parent/groupId" => cur_p_g = val.clone(),
                    "project/parent/artifactId" => cur_p_a = val.clone(),
                    "project/parent/version" => cur_p_v = val.clone(),
                    "project/repositories/repository/url" => cur_repo_url = val.clone(),
                    _ => {}
                }

                if full_path.starts_with("project/dependencies/dependency/")
                    || full_path.starts_with("project/dependencyManagement/dependencies/dependency/")
                {
                    match full_path.rsplit('/').next().unwrap() {
                        "groupId" => cur_g = val.clone(),
                        "artifactId" => cur_a = val.clone(),
                        "version" => cur_v = val.clone(),
                        "scope" => cur_s = val.clone(),
                        "type" => cur_t = val.clone(),
                        _ => {}
                    }
                }
            }

            Ok(Event::End(_)) => {
                let full_path = path.join("/");

                if full_path == "project/parent" && !cur_p_g.is_empty() && !cur_p_a.is_empty() && !cur_p_v.is_empty() {
                    pom.parent = Some(Artifact::new(&cur_p_g, &cur_p_a, &cur_p_v));
                    cur_p_g.clear();
                    cur_p_a.clear();
                    cur_p_v.clear();
                } else if full_path == "project/repositories/repository" && !cur_repo_url.is_empty() {
                    pom.repositories.push(cur_repo_url.clone());
                    cur_repo_url.clear();
                } else if full_path == "project/dependencies/dependency"
                    || full_path == "project/dependencyManagement/dependencies/dependency"
                {
                    if cur_g.is_empty() || cur_a.is_empty() || cur_a == "*" {
                        // Пропускаем invalid или wildcard зависимости из BOM
                        cur_g.clear();
                        cur_a.clear();
                        cur_v.clear();
                        cur_s.clear();
                        cur_t.clear();
                        continue;
                    }

                    let artifact = Artifact::new(&cur_g, &cur_a, &cur_v);
                    let entry = DependencyEntry {
                        artifact,
                        scope: if cur_s.is_empty() { None } else { Some(cur_s.clone()) },
                        entry_type: if cur_t.is_empty() { None } else { Some(cur_t.clone()) },
                    };

                    if section == "dependencies" {
                        pom.dependencies.push(entry);
                    } else if section == "dep_management" {
                        pom.dep_management.push(entry);
                    }

                    cur_g.clear();
                    cur_a.clear();
                    cur_v.clear();
                    cur_s.clear();
                    cur_t.clear();
                } else if full_path == "project/dependencies"
                    || full_path == "project/dependencyManagement/dependencies"
                {
                    section.clear();
                }

                path.pop();
            }

            Ok(Event::Eof) => break,
            _ => (),
        }

        buf.clear();
    }

    pom
}

pub fn process_imports(dep_man: &mut Vec<DependencyEntry>, resolver: &Resolver) {
    let mut i = 0;
    while i < dep_man.len() {
        let entry = dep_man[i].clone();
        if entry.entry_type.as_deref() == Some("pom") && entry.scope.as_deref() == Some("import") {
            if let Ok(pom_path) = resolver.fetch_artifact(&entry.artifact, "pom") {
                if let Ok(xml) = fs::read_to_string(pom_path) {
                    let imported = parse(&xml, &entry.artifact);
                    dep_man.extend(imported.dep_management);
                }
            }
            dep_man.remove(i);
        } else {
            i += 1;
        }
    }
}

pub fn resolve_val(val: &str, props: &HashMap<String, String>, art: &Artifact) -> String {
    if val == "${project.groupId}" || val == "${groupId}" {
        return art.group.clone();
    }
    if val == "${project.version}" || val == "${version}" {
        return art.version.clone();
    }

    let mut result = val.to_string();
    for (k, v) in props {
        let pattern = format!("${{{}}}", k);
        result = result.replace(&pattern, v);
    }
    result
}