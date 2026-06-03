use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize)]
pub struct TsConfig {
    #[serde(default, rename = "compilerOptions")]
    pub compiler_options: Option<TsCompilerOptions>,
    pub files: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct TsCompilerOptions {
    pub root_dir: Option<String>,
    pub base_url: Option<String>,
    pub paths: Option<BTreeMap<String, Vec<String>>>,
    pub module_resolution: Option<String>,
}

pub fn load_tsconfig(path: &Path) -> Result<(TsConfig, PathBuf)> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("lendo tsconfig em {:?}", path))?;
    let mut stripped = Vec::new();
    json_comments::StripComments::new(content.as_bytes())
        .read_to_end(&mut stripped)
        .map_err(|e| anyhow!("removendo comentarios de {:?}: {}", path, e))?;
    let tsconfig: TsConfig = serde_json::from_slice(&stripped)
        .with_context(|| format!("parseando tsconfig em {:?}", path))?;
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    Ok((tsconfig, dir))
}

pub fn is_relevant_ts_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.ends_with(".d.ts") {
        return false;
    }
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("ts") | Some("tsx") | Some("mts") | Some("cts")
    )
}

fn normalize_exclude(raw: &str) -> String {
    if raw.contains('*') || raw.contains('/') || raw.contains('\\') {
        raw.to_string()
    } else {
        format!("**/{}/**", raw)
    }
}

fn normalize_include(raw: &str) -> String {
    if raw.contains('*') {
        raw.to_string()
    } else if raw.contains('/') || raw.contains('\\') {
        let trimmed = raw.trim_end_matches('/').trim_end_matches('\\');
        format!("{}/**/*", trimmed)
    } else {
        format!("{}/**/*", raw)
    }
}

pub fn discover_files(tsconfig: &TsConfig, tsconfig_dir: &Path) -> Result<Vec<PathBuf>> {
    if let Some(explicit) = &tsconfig.files {
        let mut files = Vec::new();
        for f in explicit {
            let p = tsconfig_dir.join(f);
            if p.exists() && is_relevant_ts_file(&p) {
                files.push(p);
            }
        }
        files.sort();
        return Ok(files);
    }

    let include_patterns: Vec<String> = tsconfig
        .include
        .as_ref()
        .cloned()
        .unwrap_or_else(|| vec!["**/*.ts".into(), "**/*.tsx".into()])
        .iter()
        .map(|p| normalize_include(p))
        .collect();

    let exclude_patterns = tsconfig
        .exclude
        .as_ref()
        .cloned()
        .unwrap_or_else(|| vec!["node_modules".into(), ".annotated".into()]);

    let normalized_excludes: Vec<String> =
        exclude_patterns.iter().map(|e| normalize_exclude(e)).collect();

    let exclude_dir_names: Vec<&str> = exclude_patterns
        .iter()
        .filter(|p| !p.contains('*') && !p.contains('/') && !p.contains('\\'))
        .map(|s| s.as_str())
        .collect();

    let mut inc_builder = globset::GlobSetBuilder::new();
    for inc in &include_patterns {
        inc_builder.add(globset::Glob::new(inc)?);
    }
    let mut exc_builder = globset::GlobSetBuilder::new();
    for exc in &normalized_excludes {
        exc_builder.add(globset::Glob::new(exc)?);
    }
    let inc_set = inc_builder.build()?;
    let exc_set = exc_builder.build()?;

    let mut files = Vec::new();
    let mut it = walkdir::WalkDir::new(tsconfig_dir).into_iter();
    while let Some(entry) = it.next() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if exclude_dir_names.contains(&name) {
                    it.skip_current_dir();
                    continue;
                }
            }
        }

        if !path.is_file() || !is_relevant_ts_file(path) {
            continue;
        }
        let rel = path
            .strip_prefix(tsconfig_dir)
            .unwrap_or(path)
            .to_string_lossy();
        if !inc_set.is_match(rel.as_ref()) {
            continue;
        }
        if exc_set.is_match(rel.as_ref()) {
            continue;
        }
        files.push(path.to_path_buf());
    }

    files.sort();
    Ok(files)
}
