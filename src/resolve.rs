use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use swc_ecma_visit::Visit;

use crate::tsconfig::TsConfig;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExportKey {
    pub file: PathBuf,
    pub name: String,
}

pub type UsageMap = BTreeMap<ExportKey, BTreeSet<PathBuf>>;

fn try_resolve_candidate(candidate: &Path) -> Option<PathBuf> {
    let extensions = [".ts", ".tsx", ".mts", ".cts"];
    for ext in &extensions {
        let p = candidate.with_extension(&ext[1..]);
        if p.exists() {
            return Some(p.canonicalize().ok()?);
        }
        let p = PathBuf::from(format!("{}{}", candidate.to_string_lossy(), ext));
        if p.exists() {
            return Some(p.canonicalize().ok()?);
        }
    }
    let index = candidate.join("index");
    for ext in &extensions {
        let p = index.with_extension(&ext[1..]);
        if p.exists() {
            return Some(p.canonicalize().ok()?);
        }
    }
    None
}

pub fn resolve_module_specifier(
    specifier: &str,
    from_file: &Path,
    tsconfig_dir: &Path,
    tsconfig: &TsConfig,
) -> Option<PathBuf> {
    if specifier.starts_with('.') {
        let from_dir = from_file.parent().unwrap_or(tsconfig_dir);
        let candidate = from_dir.join(specifier);
        return try_resolve_candidate(&candidate);
    }

    let compiler_options = tsconfig.compiler_options.as_ref();
    let base_url = compiler_options
        .and_then(|co| co.base_url.as_ref())
        .map(|bu| tsconfig_dir.join(bu));

    if let Some(base) = &base_url {
        let candidate = base.join(specifier);
        if let Some(resolved) = try_resolve_candidate(&candidate) {
            return Some(resolved);
        }
    }

    if let Some(paths) = compiler_options.and_then(|co| co.paths.as_ref()) {
        let base = base_url.as_deref().unwrap_or(tsconfig_dir);
        for (pattern, targets) in paths {
            if let Some(wildcard_match) = path_pattern_match(pattern, specifier) {
                for target in targets {
                    let replaced = target.replace('*', wildcard_match);
                    let candidate = base.join(&replaced);
                    if let Some(resolved) = try_resolve_candidate(&candidate) {
                        return Some(resolved);
                    }
                }
            }
        }
    }

    None
}

fn path_pattern_match<'a>(pattern: &str, specifier: &'a str) -> Option<&'a str> {
    if !pattern.contains('*') {
        return if specifier == pattern {
            Some("")
        } else {
            None
        };
    }
    let parts: Vec<&str> = pattern.splitn(2, '*').collect();
    let prefix = parts[0];
    let suffix = parts.get(1).copied().unwrap_or("");
    if specifier.starts_with(prefix) && specifier.ends_with(suffix) {
        let start = prefix.len();
        let end = specifier.len().saturating_sub(suffix.len());
        if start <= end {
            return Some(&specifier[start..end]);
        }
    }
    None
}

pub fn build_usage_map(
    source_files: &[PathBuf],
    tsconfig_dir: &Path,
    tsconfig: &TsConfig,
) -> (UsageMap, Vec<crate::collect_exports::ExportEntry>) {
    let mut usage: UsageMap = BTreeMap::new();
    let mut all_exports: Vec<crate::collect_exports::ExportEntry> = Vec::new();
    let mut namespace_usage: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut all_exports_by_file: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();

    for file in source_files {
        let parsed = match crate::parser_mod::parse_ts_file(file) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let canonical = file.canonicalize().unwrap_or_else(|_| file.clone());

        let sm = parsed.source_map.clone();
        let mut collector = crate::collect_exports::ExportCollector::new(
            canonical.clone(),
            move |pos: swc_common::BytePos| sm.lookup_char_pos(pos).line,
        );
        collector.visit_module(&parsed.module);
        for entry in &collector.exports {
            all_exports_by_file
                .entry(canonical.clone())
                .or_default()
                .push(entry.key.name.clone());
        }
        all_exports.extend(collector.exports);

        let mut import_collector = crate::collect_imports::ImportCollector::new();
        import_collector.visit_module(&parsed.module);

        for info in &import_collector.imports {
            let resolved =
                resolve_module_specifier(&info.specifier, file, tsconfig_dir, tsconfig);
            let resolved = match resolved {
                Some(r) => r,
                None => continue,
            };

            if info.default_import {
                let key = ExportKey {
                    file: resolved.clone(),
                    name: "default".to_string(),
                };
                usage.entry(key).or_default().insert(canonical.clone());
            }

            for named in &info.named_imports {
                let key = ExportKey {
                    file: resolved.clone(),
                    name: named.clone(),
                };
                usage.entry(key).or_default().insert(canonical.clone());
            }

            if info.namespace_import {
                namespace_usage
                    .entry(resolved.clone())
                    .or_default()
                    .insert(canonical.clone());
            }
        }
    }

    for (module_path, consumers) in &namespace_usage {
        if let Some(exports) = all_exports_by_file.get(module_path) {
            for export_name in exports {
                let key = ExportKey {
                    file: module_path.clone(),
                    name: export_name.clone(),
                };
                for consumer in consumers {
                    usage.entry(key.clone()).or_default().insert(consumer.clone());
                }
            }
        }
    }

    (usage, all_exports)
}
