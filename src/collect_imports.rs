use swc_atoms::Atom;
use swc_ecma_ast::*;
use swc_ecma_visit::Visit;

pub struct ImportInfo {
    pub specifier: String,
    pub default_import: bool,
    pub named_imports: Vec<String>,
    pub namespace_import: bool,
}

pub struct ImportCollector {
    pub imports: Vec<ImportInfo>,
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

impl ImportCollector {
    pub fn new() -> Self {
        Self {
            imports: Vec::new(),
        }
    }
}

impl Visit for ImportCollector {
    fn visit_module(&mut self, module: &Module) {
        for item in &module.body {
            if let ModuleItem::ModuleDecl(decl) = item {
                self.collect_from_module_decl(decl);
            }
        }
    }
}

impl ImportCollector {
    fn collect_from_module_decl(&mut self, decl: &ModuleDecl) {
        match decl {
            ModuleDecl::Import(import_decl) => {
                let specifier = wtf8_str(&import_decl.src.value);
                let mut default_import = false;
                let mut named_imports = Vec::new();
                let mut namespace_import = false;

                for spec in &import_decl.specifiers {
                    match spec {
                        ImportSpecifier::Default(_) => {
                            default_import = true;
                        }
                        ImportSpecifier::Named(named) => {
                            let name = named
                                .imported
                                .as_ref()
                                .map(module_export_name_str)
                                .unwrap_or_else(|| atom_str(&named.local.sym));
                            named_imports.push(name);
                        }
                        ImportSpecifier::Namespace(_) => {
                            namespace_import = true;
                        }
                    }
                }

                self.imports.push(ImportInfo {
                    specifier,
                    default_import,
                    named_imports,
                    namespace_import,
                });
            }
            ModuleDecl::ExportNamed(export_named) => {
                if let Some(src) = &export_named.src {
                    let specifier = wtf8_str(&src.value);
                    let mut named_imports = Vec::new();
                    for spec in &export_named.specifiers {
                        let name = match spec {
                            ExportSpecifier::Named(ns) => {
                                module_export_name_str(&ns.orig)
                            }
                            ExportSpecifier::Default(ds) => {
                                atom_str(&ds.exported.sym)
                            }
                            ExportSpecifier::Namespace(ns) => {
                                module_export_name_str(&ns.name)
                            }
                        };
                        named_imports.push(name);
                    }
                    self.imports.push(ImportInfo {
                        specifier,
                        default_import: false,
                        named_imports,
                        namespace_import: false,
                    });
                }
            }
            _ => {}
        }
    }
}
