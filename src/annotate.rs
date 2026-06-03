use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::collect_exports::ExportEntry;
use crate::resolve::UsageMap;

fn relative_for_display(from: &Path, to: &Path) -> String {
    let rel = pathdiff::diff_paths(to, from.parent().unwrap_or(from))
        .unwrap_or_else(|| to.to_path_buf());
    let s = rel.to_string_lossy().replace('\\', "/");
    if s.starts_with('.') || s.starts_with('/') {
        s
    } else {
        format!("./{}", s)
    }
}

fn build_comment(export_name: &str, consumer_paths: &[PathBuf], target_file: &Path) -> String {
    let mut rels: Vec<String> = consumer_paths
        .iter()
        .map(|p| relative_for_display(target_file, p))
        .collect();
    rels.sort();
    rels.dedup();
    format!(
        "/* Export {} usado por: {} */",
        export_name,
        rels.join(", ")
    )
}

pub fn annotate_file(
    file_path: &Path,
    exports: &[ExportEntry],
    usage: &UsageMap,
) -> std::io::Result<Option<String>> {
    let content = std::fs::read_to_string(file_path)?;
    let lines: Vec<&str> = content.lines().collect();

    let mut annotations: BTreeMap<usize, Vec<String>> = BTreeMap::new();

    for entry in exports {
        let consumers = match usage.get(&entry.key) {
            Some(c) if !c.is_empty() => c,
            _ => continue,
        };

        let export_name = &entry.key.name;
        let comment = build_comment(
            export_name,
            &consumers.iter().cloned().collect::<Vec<_>>(),
            file_path,
        );

        let target_line = if entry.line > 0 {
            entry.line - 1
        } else {
            0
        };
        let target_line = target_line.min(lines.len().saturating_sub(1));

        let line_text = lines.get(target_line).unwrap_or(&"");
        if line_text.contains(&format!("Export {} usado por:", export_name)) {
            continue;
        }

        let insert_line = target_line;
        if insert_line > 0 {
            let prev = lines.get(insert_line - 1).unwrap_or(&"");
            if prev.contains(&format!("Export {} usado por:", export_name)) {
                continue;
            }
        }

        annotations
            .entry(insert_line)
            .or_default()
            .push(comment);
    }

    if annotations.is_empty() {
        return Ok(None);
    }

    let mut result_lines = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if let Some(comments) = annotations.get(&i) {
            for comment in comments {
                result_lines.push(comment.clone());
            }
        }
        result_lines.push(line.to_string());
    }

    let mut result = result_lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }

    Ok(Some(result))
}

pub fn write_output(
    input_file: &Path,
    output_text: &str,
    in_place: bool,
    out_dir: Option<&Path>,
    tsconfig_dir: &Path,
) -> std::io::Result<PathBuf> {
    if in_place {
        std::fs::write(input_file, output_text)?;
        return Ok(input_file.to_path_buf());
    }

    let out_dir = out_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| tsconfig_dir.join(".annotated"));

    let rel = pathdiff::diff_paths(input_file, tsconfig_dir)
        .unwrap_or_else(|| input_file.to_path_buf());
    let out_file = out_dir.join(&rel);

    if let Some(parent) = out_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_file, output_text)?;
    Ok(out_file)
}
