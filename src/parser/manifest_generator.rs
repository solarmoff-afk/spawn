// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

use std::io::Cursor;
use std::path::Path;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

use crate::parser::toml_parser::Config;

/// Эта функция получает путь к манифесту, конфиг (который получается через парсинг .toml файлов)
/// и заменяет/добавляет поля пакета, версии и прочего в xml создавая новый манифест.
/// Система сборки должна сохранить этот манифест и позже использовать на этапе
/// компиляции и линковки ресурсов
pub fn generate_manifest(template_path: &Path, config: &Config) -> Result<String, Box<dyn std::error::Error>> {
    let xml_content = std::fs::read_to_string(template_path)?;

    let mut reader = Reader::from_str(&xml_content);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    let pkg_info = config.package.as_ref();
    let mut uses_sdk_inserted = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"manifest" => {
                let mut elem = e.clone();
                
                if let Some(p) = pkg_info.and_then(|i| i.package.as_ref()) {
                    update_or_add_attr(&mut elem, b"package", p);
                }
                
                if let Some(v) = pkg_info.and_then(|i| i.version.as_ref()) {
                    update_or_add_attr(&mut elem, b"android:versionName", v);
                }
                
                if let Some(vc) = pkg_info.and_then(|i| i.version_code) {
                    update_or_add_attr(&mut elem, b"android:versionCode", &vc.to_string());
                }
                
                writer.write_event(Event::Start(elem))?;
            }

            Ok(Event::Start(e)) if e.name().as_ref() == b"uses-sdk" => {
                let mut elem = e.clone();

                apply_sdk_attrs(&mut elem, config);
                writer.write_event(Event::Start(elem))?;
                uses_sdk_inserted = true;
            }

            Ok(Event::Empty(e)) if e.name().as_ref() == b"uses-sdk" => {
                let mut elem = e.clone();
                
                apply_sdk_attrs(&mut elem, config);
                writer.write_event(Event::Empty(elem))?;
                uses_sdk_inserted = true;
            }

            Ok(Event::Start(e)) if e.name().as_ref() == b"application" => {
                if !uses_sdk_inserted {
                    if let Some(sdk_tag) = create_sdk_tag(config) {
                        writer.write_event(Event::Empty(sdk_tag))?;
                    }

                    uses_sdk_inserted = true;
                }

                let mut elem = e.clone();
                
                if let Some(l) = pkg_info.and_then(|i| i.label.as_ref()) {
                    update_or_add_attr(&mut elem, b"android:label", l);
                }
                
                if let Some(i) = pkg_info.and_then(|i| i.icon.as_ref()) {
                    update_or_add_attr(&mut elem, b"android:icon", i);
                }
                
                writer.write_event(Event::Start(elem))?;
            }

            Ok(Event::Eof) => break,
            Ok(e) => { writer.write_event(e)?; }
            Err(e) => return Err(Box::new(e)),
        }

        buf.clear();
    }

    let result = writer.into_inner().into_inner();
    Ok(String::from_utf8(result)?)
}

/// Эта функция создаёт блок uses-sdk в манифесте
fn apply_sdk_attrs(elem: &mut BytesStart, config: &Config) {
    if let Some(p) = &config.package {
        if let Some(min) = p.min_sdk {
            update_or_add_attr(elem, b"android:minSdkVersion", &min.to_string());
        }

        if let Some(target) = p.target_sdk {
            update_or_add_attr(elem, b"android:targetSdkVersion", &target.to_string());
        }
    }
}

fn create_sdk_tag(config: &Config) -> Option<BytesStart<'static>> {
    let mut elem = BytesStart::new("uses-sdk");
    apply_sdk_attrs(&mut elem, config);

    if elem.attributes().count() > 0 {
        Some(elem)
    } else {
        None
    }
}

fn update_or_add_attr(elem: &mut BytesStart, name: &[u8], value: &str) {
    let preserved_attrs: Vec<(Vec<u8>, Vec<u8>)> = elem.attributes()
        .filter_map(|a| {
            let a = a.ok()?;

            if a.key.as_ref() == name { 
                None
            } else {
                Some((a.key.as_ref().to_vec(), a.value.as_ref().to_vec()))
            }
        }).collect();

    elem.clear_attributes();

    for (k, v) in preserved_attrs {
        elem.push_attribute((k.as_slice(), v.as_slice()));
    }

    elem.push_attribute((name, value.as_bytes()));
}