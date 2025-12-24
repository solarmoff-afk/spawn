// Copyright (c) 2025 Spawn
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// https://www.eclipse.org/legal/epl-2.0/
// SPDX-License-Identifier: EPL-2.0

use std::fs;
use walkdir::WalkDir;

use crate::parser::Config;
use crate::resolver::Resolver;

pub fn generate_ninja(
    config: &Config,
    resolver: Option<&Resolver>,
    
    // Формат выходного файла, тут либо apk либо aab
    output_type: &str, 
) -> Result<(), Box<dyn std::error::Error>> {
    task!("Generate build.ninja");

    let build_dir = config.base_path.join(".spawn").join("build");
    let cache_dir = config.base_path.join(".spawn").join("cache");
    let ninja_path = build_dir.join("build.ninja");

    fs::create_dir_all(&build_dir)?;

    let mut ninja = String::new();

    ninja.push_str(&format!("builddir = {}\n", build_dir.display()));
    ninja.push_str(&format!("cachedir = {}\n", cache_dir.display()));
    ninja.push_str("\n");

    let target_sdk = config.package.as_ref()
        .and_then(|p| p.target_sdk)
        .unwrap_or(34);
    
    ninja.push_str(&format!("android_jar = $ANDROID_HOME/platforms/android-{}/android.jar\n", target_sdk));
    ninja.push_str("javac = javac\n");
    ninja.push_str("kotlinc = kotlinc\n");
    ninja.push_str("aapt2 = aapt2\n");
    ninja.push_str("d8 = d8\n");
    ninja.push_str("zip = zip\n");
    ninja.push_str("zipalign = zipalign\n");
    ninja.push_str("apksigner = apksigner\n");
    ninja.push_str("bundletool = java -jar $BUNDLETOOL_JAR\n\n");

    ninja.push_str("rule javac\n");
    ninja.push_str("  command = $javac -d $outdir -classpath $classpath -source 1.8 -target 1.8 $in\n");
    ninja.push_str("  description = JAVAC $in\n\n");

    ninja.push_str("rule kotlinc\n");
    ninja.push_str("  command = $kotlinc -d $outdir -classpath $classpath -jvm-target 1.8 $in\n");
    ninja.push_str("  description = KOTLINC $in\n\n");

    ninja.push_str("rule aapt2_compile\n");
    ninja.push_str("  command = $aapt2 compile --dir $in -o $out\n");
    ninja.push_str("  description = AAPT2 compile $in\n\n");

    ninja.push_str("rule aapt2_link\n");
    ninja.push_str("  command = $aapt2 link -o $out --manifest $manifest -I $android_jar --auto-add-overlay $in\n");
    ninja.push_str("  description = AAPT2 link\n\n");

    ninja.push_str("rule d8\n");
    ninja.push_str("  command = $d8 --release --output $out $in\n");
    ninja.push_str("  description = D8/R8 optimization\n\n");

    ninja.push_str("rule package_apk\n");
    ninja.push_str("  command = cd $builddir && $zip -r $out . && cd -\n");
    ninja.push_str("  description = Packaging unsigned APK\n\n");

    ninja.push_str("rule zipalign\n");
    ninja.push_str("  command = $zipalign -f -v 4 $in $out\n");
    ninja.push_str("  description = Aligning APK\n\n");

    ninja.push_str("rule apksigner\n");
    ninja.push_str("  command = $apksigner sign --ks $keystore --ks-key-alias $alias --out $out $in\n");
    ninja.push_str("  description = Signing APK\n\n");

    ninja.push_str("rule build_aab\n");
    ninja.push_str("  command = $bundletool build-bundle --modules $modules_dir --output $out\n");
    ninja.push_str("  description = Building AAB\n\n");

    let mut classpath = String::from("$android_jar");
    if let Some(r) = resolver {
        for (_, art) in &r.resolved_artifacts {
            let unpacked = r.cache_root.join(art.get_path("unpacked"));
            let classes_jar = unpacked.join("classes.jar");

            if classes_jar.exists() {
                classpath.push_str(&format!(":{}", classes_jar.display()));
            }
        }
    }
    ninja.push_str(&format!("classpath = {}\n\n", classpath));

    let mut module_dirs = Vec::new();
    for module_path in &config.modules {
        let dir = module_path.parent().unwrap().to_path_buf();
        module_dirs.push(dir);
    }

    let mut all_classes_dirs = Vec::new();
    let mut all_flat_resources = Vec::new();

    // Компиляция каждого модуля, все модули компилируются и потом линкуются
    for module_dir in &module_dirs {
        let module_name = module_dir.file_name().unwrap().to_string_lossy();

        let java_dir = module_dir.join("java");
        let kotlin_dir = module_dir.join("kotlin");
        let res_dir = module_dir.join("res");

        let module_out_dir = build_dir.join(format!("{}_out", module_name));
        let module_classes_dir = module_out_dir.join("classes");
        let module_flat_res = build_dir.join(format!("{}_flat.res", module_name));

        all_classes_dirs.push(module_classes_dir.clone());

        // Java
        let mut java_sources = String::new();
        if java_dir.exists() {
            for entry in WalkDir::new(&java_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().extension().map_or(false, |e| e == "java") {
                    java_sources.push_str(&format!("{} ", entry.path().display()));
                }
            }

            if !java_sources.is_empty() {
                ninja.push_str(&format!(
                    "build {}: javac {}\n",
                    module_classes_dir.display(),
                    java_sources.trim()
                ));

                ninja.push_str("  classpath = $classpath\n");
                ninja.push_str(&format!("  outdir = {}\n\n", module_classes_dir.display()));
            }
        }

        // Kotlin
        let mut kotlin_sources = String::new();
        if kotlin_dir.exists() {
            for entry in WalkDir::new(&kotlin_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().extension().map_or(false, |e| e == "kt") {
                    kotlin_sources.push_str(&format!("{} ", entry.path().display()));
                }
            }

            if !kotlin_sources.is_empty() {
                ninja.push_str(&format!(
                    "build {}: kotlinc {}\n",
                    module_classes_dir.display(),
                    kotlin_sources.trim()
                ));

                ninja.push_str("  classpath = $classpath\n");
                ninja.push_str(&format!("  outdir = {}\n\n", module_classes_dir.display()));
            }
        }

        // И для ресурсов компиляция
        if res_dir.exists() {
            ninja.push_str(&format!(
                "build {}: aapt2_compile {}\n",
                module_flat_res.display(),
                res_dir.display()
            ));

            all_flat_resources.push(module_flat_res);
        }
    }

    // Линковка тут обзая для всех модулей
    let linked_resources = build_dir.join("linked_resources.ap_");
    if !all_flat_resources.is_empty() {
        let inputs = all_flat_resources.iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" ");

        ninja.push_str(&format!(
            "build {}: aapt2_link {}\n",
            linked_resources.display(),
            inputs
        ));

        ninja.push_str(&format!("  manifest = {}\n", cache_dir.join("AndroidManifest.xml").display()));
        ninja.push_str("\n");
    }

    // Дексинг
    let dex_dir = build_dir.join("dex");
    let dex_inputs = all_classes_dirs.iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(" ");

    if !dex_inputs.is_empty() {
        ninja.push_str(&format!(
            "build {}: d8 {}\n",
            dex_dir.display(),
            dex_inputs
        ));

        ninja.push_str("\n");
    }

    // Финальная сборка
    let unsigned_apk = build_dir.join("unsigned.apk");
    let aligned_apk = build_dir.join("aligned.apk");
    let signed_apk = build_dir.join("app.apk");

    if output_type == "apk" {
        ninja.push_str(&format!(
            "build {}: package_apk {}\n",
            unsigned_apk.display(),
            dex_dir.display()
        ));

        if linked_resources.exists() {
            ninja.push_str(&format!("  resources = {}\n", linked_resources.display()));
        }

        ninja.push_str("\n");

        ninja.push_str(&format!(
            "build {}: zipalign {}\n",
            aligned_apk.display(),
            unsigned_apk.display()
        ));

        ninja.push_str("\n");

        ninja.push_str(&format!(
            "build {}: apksigner {}\n",
            signed_apk.display(),
            aligned_apk.display()
        ));

        if let Some(sign) = &config.sign {
            ninja.push_str(&format!("  keystore = {}\n", sign.keystore));
            ninja.push_str(&format!("  alias = {}\n", sign.alias));
        }

        ninja.push_str("\n");

        ninja.push_str(&format!("default {}\n", signed_apk.display()));
    } else {
        let aab_output = build_dir.join("app.aab");
        let modules_input = module_dirs.iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" ");

        ninja.push_str(&format!(
            "build {}: build_aab {}\n",
            aab_output.display(),
            modules_input
        ));

        ninja.push_str(&format!("default {}\n", aab_output.display()));
    }

    fs::write(&ninja_path, ninja)?;
    
    Ok(())
}