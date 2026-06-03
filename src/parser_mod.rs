use anyhow::Result;
use std::path::Path;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::Module;
use swc_ecma_parser::{lexer::Lexer, Parser as SwcParser, StringInput, Syntax, TsSyntax};

pub struct ParsedFile {
    pub module: Module,
    pub source_map: Lrc<SourceMap>,
}

pub fn parse_ts_file(file_path: &Path) -> Result<ParsedFile> {
    let cm: Lrc<SourceMap> = Default::default();
    let content = std::fs::read_to_string(file_path)?;

    let fm = cm.new_source_file(
        FileName::Real(file_path.to_path_buf()).into(),
        content,
    );

    let is_tsx = file_path.extension().is_some_and(|e| e == "tsx");

    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: is_tsx,
            decorators: true,
            ..Default::default()
        }),
        swc_ecma_ast::EsVersion::Es2020,
        StringInput::from(&*fm),
        None,
    );

    let mut parser = SwcParser::new_from(lexer);
    let module = parser
        .parse_module()
        .map_err(|e| anyhow::anyhow!("parse error in {:?}: {:?}", file_path, e))?;

    Ok(ParsedFile {
        module,
        source_map: cm,
    })
}
