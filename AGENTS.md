# AGENTS.md

## Build & Verify

```sh
cargo check          # compile check (zero warnings expected)
cargo run -- --project <tsconfig-path> --dry-run
cargo run -- --project <tsconfig-path> --write --out-dir <dir>
cargo run -- --project <tsconfig-path> --write --in-place
```

No test suite yet — manual verification against `tests/fixtures/` projects.

## Architecture

Single binary, no workspace. Entry: `src/main.rs` orchestrates the pipeline:

1. `tsconfig` — loads tsconfig.json, discovers TS files via globset+walkdir
2. `parser_mod` — SWC parse per file (creates a **fresh `SourceMap` per file** — sharing one causes BytePos offsets to accumulate, producing wrong line numbers)
3. `collect_exports` — `ExportCollector` visitor; receives line lookup as `Box<dyn Fn(BytePos) -> usize>` closure
4. `collect_imports` — `ImportCollector` visitor; also records re-exports (`export { X } from './y'`) as imports
5. `resolve` — module resolution (relative, baseUrl, paths wildcards), builds `UsageMap`, namespace import expansion
6. `annotate` — inserts `/* Export X usado por: ... */` comments before export lines, writes output

## SWC API Pitfalls

These cost significant debugging time:

- **Per-file SourceMap is mandatory**: `parse_ts_file` creates its own `Lrc<SourceMap>` internally. Never share one across files — BytePos offsets accumulate and `lookup_char_pos` returns wrong lines.
- **Line lookup via closure**: `ExportCollector::new(file, closure)` takes `Box<dyn Fn(BytePos) -> usize>`, not a `Vec<usize>`. The closure captures a cloned `Lrc<SourceMap>`.
- **SWC version**: using swc_ecma_ast=23, swc_ecma_parser=39, swc_common=21, swc_atoms=9 (v2+ era, not v1). Many online examples target v1 with different APIs.
- **`ExportSpecifier`** is an enum: `Named(ExportNamedSpecifier)`, `Default(ExportDefaultSpecifier)`, `Namespace(ExportNamespaceSpecifier)` — not a flat struct.
- **`TsModuleDeclId`** → `TsModuleName` enum with `Ident`/`Str` variants.
- **`Decl::Using`** variant exists in this SWC version (handle with empty name).
- **`Str.value`** is `Wtf8Atom` (no Display) → use `.to_string_lossy().into_owned()`.
- **`Ident.sym`** is `Atom` (has Display) → use `.to_string()`.
- **`new_source_file`** accepts owned `String` directly (via `String: Into<BytesStr>`).

## Conventions

- UI strings and comments are in Portuguese: `/* Export {name} usado por: {paths} */`
- `canonicalize()` is used for file identity — all ExportKey.file paths are canonical. Import resolution must also produce canonical paths for matching to work.
- Re-exports (`export { X } from './y'`) are tracked as imports from the source file, but `index.ts` re-exports are not registered as exports themselves (only source file exports are annotated). Transitve tracking through re-export barrels is not implemented.
- `include` patterns in tsconfig are normalized: plain directory names like `"src"` are expanded to `src/**/*` (matching TypeScript's behavior). Patterns with globs (`*`) are kept as-is.
- `ParsedFile.source_file` field was removed — don't add it back; it caused an Arc/Lrc type mismatch and wasn't needed.

## Test Fixtures

`tests/fixtures/` contains three TS projects for manual testing:

- `ts-test-project/` — basic named + default imports
- `ns-test-project/` — namespace import (`import * as X`) expansion
- `reexport-test-project/` — re-export tracking (`export { X } from './y'`)

Run against any fixture:
```sh
cargo run -- --project tests/fixtures/<name>/tsconfig.json --dry-run
```

`.annotated/` dirs in fixtures are gitignored — safe to write there during testing.
