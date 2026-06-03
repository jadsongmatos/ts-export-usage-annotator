use clap::Parser;

#[derive(Parser)]
#[command(name = "ts-export-usage-annotator")]
#[command(about = "Varre um projeto TypeScript, encontra exports e adiciona comentarios com os caminhos dos arquivos que os utilizam.")]
pub struct Cli {
    #[arg(long, short = 'p', default_value = "tsconfig.json")]
    pub project: std::path::PathBuf,

    #[arg(long, group = "mode")]
    pub write: bool,

    #[arg(long, group = "mode")]
    pub dry_run: bool,

    #[arg(long)]
    pub in_place: bool,

    #[arg(long)]
    pub out_dir: Option<std::path::PathBuf>,
}

impl Cli {
    pub fn is_dry_run(&self) -> bool {
        !self.write
    }
}
