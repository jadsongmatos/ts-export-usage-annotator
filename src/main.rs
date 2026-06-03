mod annotate;
mod cli;
mod collect_exports;
mod collect_imports;
mod parser_mod;
mod resolve;
mod tsconfig;

use anyhow::{bail, Result};
use clap::Parser;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    if cli.write && cli.dry_run {
        bail!("Use apenas um modo: --write ou --dry-run.");
    }
    let dry_run = cli.is_dry_run();

    let tsconfig_path = &cli.project;
    if !tsconfig_path.exists() {
        bail!("tsconfig nao encontrado em {:?}", tsconfig_path);
    }

    let (tsconfig, tsconfig_dir) = tsconfig::load_tsconfig(tsconfig_path)?;
    let source_files = tsconfig::discover_files(&tsconfig, &tsconfig_dir)?;

    if source_files.is_empty() {
        println!("Nenhum arquivo TypeScript encontrado.");
        return Ok(());
    }

    let (usage, all_exports) =
        resolve::build_usage_map(&source_files, &tsconfig_dir, &tsconfig);

    let mut exports_by_file: BTreeMap<PathBuf, Vec<collect_exports::ExportEntry>> =
        BTreeMap::new();
    for entry in all_exports {
        exports_by_file
            .entry(entry.key.file.clone())
            .or_default()
            .push(entry);
    }

    let mut changed = 0u32;

    for (file_path, exports) in &exports_by_file {
        let display_path = file_path.to_string_lossy();

        let annotated = annotate::annotate_file(file_path, exports, &usage)
            .map_err(|e| anyhow::anyhow!("erro ao anotar {:?}: {}", file_path, e))?;

        let new_content = match annotated {
            Some(c) => c,
            None => continue,
        };

        changed += 1;
        if dry_run {
            println!("[alteraria] {}", display_path);
        } else if cli.in_place {
            let out =
                annotate::write_output(file_path, &new_content, true, None, &tsconfig_dir)?;
            println!("[ok] {} (in-place)", display_path);
            let _ = out;
        } else {
            let out_dir = cli.out_dir.as_deref();
            let out = annotate::write_output(
                file_path,
                &new_content,
                false,
                out_dir,
                &tsconfig_dir,
            )?;
            println!("[ok] {} -> {}", display_path, out.to_string_lossy());
        }
    }

    if changed == 0 {
        println!("Nenhuma exportacao com usos encontrados para anotar.");
    }

    Ok(())
}
