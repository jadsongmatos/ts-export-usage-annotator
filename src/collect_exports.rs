use std::path::PathBuf;
use swc_atoms::Atom;
use swc_ecma_ast::*;
use swc_ecma_visit::Visit;

use crate::resolve::ExportKey;

pub struct ExportEntry {
    pub key: ExportKey,
    pub line: usize,
}

pub struct ExportCollector {
    pub exports: Vec<ExportEntry>,
    file: PathBuf,
    line_lookup: Box<dyn Fn(swc_common::BytePos) -> usize>,
}

impl ExportCollector {
    pub fn new<F>(file: PathBuf, line_lookup: F) -> Self
    where
        F: Fn(swc_common::BytePos) -> usize + 'static,
    {
        Self {
            exports: Vec::new(),
            file,
            line_lookup: Box::new(line_lookup),
        }
    }

    fn byte_pos_to_line(&self, pos: swc_common::BytePos) -> usize {
        (self.line_lookup)(pos)
    }
}

fn atom_str(atom: &Atom) -> String {
    atom.to_string()
}

fn wtf8_str(val: &swc_atoms::Wtf8Atom) -> String {
    val.to_string_lossy().into_owned()
}

fn module_export_name_str(name: &ModuleExportName) -> String {
    match name {
        ModuleExportName::Ident(id) => atom_str(&id.sym),
        ModuleExportName::Str(s) => wtf8_str(&s.value),
    }
}

impl Visit for ExportCollector {
    fn visit_module(&mut self, module: &Module) {
        for item in &module.body {
            if let ModuleItem::ModuleDecl(decl) = item {
                self.collect_from_module_decl(decl);
            }
        }
    }
}

impl ExportCollector {
    fn collect_from_module_decl(&mut self, decl: &ModuleDecl) {
        match decl {
            ModuleDecl::ExportDecl(export_decl) => {
                let name = match &export_decl.decl {
                    Decl::Class(c) => atom_str(&c.ident.sym),
                    Decl::Fn(f) => atom_str(&f.ident.sym),
                    Decl::Var(var) => var
                        .decls
                        .first()
                        .map(|d| match &d.name {
                            Pat::Ident(id) => atom_str(&id.id.sym),
                            _ => String::new(),
                        })
                        .unwrap_or_default(),
                    Decl::TsInterface(i) => atom_str(&i.id.sym),
                    Decl::TsTypeAlias(a) => atom_str(&a.id.sym),
                    Decl::TsEnum(e) => atom_str(&e.id.sym),
                    Decl::Using(_) => String::new(),
                    Decl::TsModule(m) => match &m.id {
                        TsModuleName::Ident(id) => atom_str(&id.sym),
                        TsModuleName::Str(s) => wtf8_str(&s.value),
                    },
                };
                if !name.is_empty() {
                    let line = self.byte_pos_to_line(export_decl.span.lo);
                    self.exports.push(ExportEntry {
                        key: ExportKey {
                            file: self.file.clone(),
                            name,
                        },
                        line,
                    });
                }
            }
            ModuleDecl::ExportDefaultDecl(export_default) => {
                let name = match &export_default.decl {
                    DefaultDecl::Class(c) => c
                        .ident
                        .as_ref()
                        .map(|id| atom_str(&id.sym))
                        .unwrap_or_else(|| "default".to_string()),
                    DefaultDecl::Fn(f) => f
                        .ident
                        .as_ref()
                        .map(|id| atom_str(&id.sym))
                        .unwrap_or_else(|| "default".to_string()),
                    DefaultDecl::TsInterfaceDecl(i) => atom_str(&i.id.sym),
                };
                let line = self.byte_pos_to_line(export_default.span.lo);
                self.exports.push(ExportEntry {
                    key: ExportKey {
                        file: self.file.clone(),
                        name: if name != "default" {
                            format!("default ({})", name)
                        } else {
                            "default".to_string()
                        },
                    },
                    line,
                });
            }
            ModuleDecl::ExportDefaultExpr(export_default_expr) => {
                let line = self.byte_pos_to_line(export_default_expr.span.lo);
                self.exports.push(ExportEntry {
                    key: ExportKey {
                        file: self.file.clone(),
                        name: "default".to_string(),
                    },
                    line,
                });
            }
            ModuleDecl::ExportNamed(export_named) => {
                if export_named.src.is_none() {
                    for spec in &export_named.specifiers {
                        let (name, span) = match spec {
                            ExportSpecifier::Named(ns) => {
                                (module_export_name_str(&ns.orig), ns.span)
                            }
                            ExportSpecifier::Default(ds) => {
                                (atom_str(&ds.exported.sym), ds.exported.span)
                            }
                            ExportSpecifier::Namespace(ns) => {
                                (module_export_name_str(&ns.name), ns.span)
                            }
                        };
                        let line = self.byte_pos_to_line(span.lo);
                        self.exports.push(ExportEntry {
                            key: ExportKey {
                                file: self.file.clone(),
                                name,
                            },
                            line,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}
