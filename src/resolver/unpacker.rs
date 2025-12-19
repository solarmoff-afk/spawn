// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

use std::fs;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub fn unpack_aar(aar_path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let out_dir = aar_path.parent().unwrap().join("unpacked");
    
    // если aar уже распакован то смысла тратить время нет
    if out_dir.exists() {
        return Ok(out_dir);
    }

    let file = fs::File::open(aar_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    fs::create_dir_all(&out_dir)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let out_path = out_dir.join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(p) = out_path.parent() {
                fs::create_dir_all(p)?;
            }

            let mut outfile = fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(out_dir)
}