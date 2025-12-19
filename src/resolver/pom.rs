// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;

use crate::resolver::artifact::Artifact;

#[derive(Debug, Default)]
pub struct Pom {
    pub properties: HashMap<String, String>,
    pub dependencies: Vec<DependencyEntry>,
}

#[derive(Debug, Clone)]
pub struct DependencyEntry {
    pub artifact: Artifact,
}

pub fn parse(xml: &str, current_art: &Artifact) -> Pom {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();

    let mut pom = Pom::default();
    let mut path = Vec::new();
    
    let mut cur_g = String::new();
    let mut cur_a = String::new();
    let mut cur_v = String::new();
    let mut cur_s = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                path.push(name);
            }

            Ok(Event::Text(e)) => {
                let val = e.unescape().unwrap_or_default().into_owned();
                let full_path = path.join("/");
                
                if full_path.starts_with("project/properties/") {
                    let key = full_path.strip_prefix("project/properties/").unwrap();
                    pom.properties.insert(key.to_string(), val.clone());
                }

                match full_path.as_str() {
                    "project/dependencies/dependency/groupId" => cur_g = val,
                    "project/dependencies/dependency/artifactId" => cur_a = val,
                    "project/dependencies/dependency/version" => cur_v = val,
                    "project/dependencies/dependency/scope" => cur_s = val,
                    _ => {}
                }
            }

            Ok(Event::End(_)) => {
                let full_path = path.join("/");

                if full_path == "project/dependencies/dependency" {
                    if cur_s != "test" && cur_s != "provided" && !cur_g.is_empty() {
                        let resolved_v = resolve_val(&cur_v, &pom.properties, current_art);
                        
                        pom.dependencies.push(DependencyEntry {
                            artifact: Artifact::new(&cur_g, &cur_a, &resolved_v),
                        });
                    }

                    cur_g.clear(); cur_a.clear(); cur_v.clear(); cur_s.clear();
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

fn resolve_val(val: &str, props: &HashMap<String, String>, art: &Artifact) -> String {
    if val == "${project.groupId}" || val == "${groupId}" { return art.group.clone(); }
    if val == "${project.version}" || val == "${version}" { return art.version.clone(); }
    
    let mut result = val.to_string();
    for (k, v) in props {
        let pattern = format!("${{{}}}", k);
        if result.contains(&pattern) {
            result = result.replace(&pattern, v);
        }
    }
    result
}