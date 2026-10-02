use std::io;
use std::path::{Path, PathBuf};

use anstyle::{AnsiColor, Effects, Style};
use anyhow::{Context, bail};
use clap::builder::Styles;
use clap::{CommandFactory, Parser, ValueEnum, ValueHint};
use clap_complete::{Shell, generate};
use indexmap::IndexMap;

use crate::rule::{Builder, Rule};
use crate::{Config, ast, oapi};

/// The commented default config: what `--init` prints.
pub const DEFAULT_CONFIG_YAML: &str = include_str!("../../config.default.yaml");

// Define custom palette and formatting styles
const STYLES: Styles = Styles::styled()
    .header(Style::new())
    .literal(AnsiColor::Cyan.on_default())
    .placeholder(Style::new().effects(Effects::DIMMED));

#[derive(Parser, Debug)]
#[command(
    name = "opi",
    about = "Generate TypeScript types from an OpenAPI spec.",
    version,
    styles = STYLES,
    color = clap::ColorChoice::Auto,
    arg_required_else_help = true
)]
pub struct Cli {
    /// OpenAPI spec to read (.json, .yaml or .yml); defaults to the config's `source`
    #[arg(value_name = "openapi", value_hint = ValueHint::FilePath)]
    pub openapi: Option<PathBuf>,

    /// File to write the config's `rules` to; stdout if omitted
    #[arg(value_name = "output", value_hint = ValueHint::FilePath)]
    pub output: Option<PathBuf>,

    /// Generator config (.yaml, .yml or .json)
    #[arg(short, long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub config: Option<PathBuf>,

    /// Print a starter config and exit
    #[arg(long, value_enum, value_name = "FORMAT", num_args = 0..=1, default_missing_value = "yaml")]
    pub init: Option<Format>,

    /// Generate shell completions and exit
    #[arg(long, value_enum, value_name = "SHELL")]
    pub completions: Option<Shell>,
}

#[derive(ValueEnum, Debug, Clone, Copy, Default)]
pub enum Format {
    #[default]
    Yaml,
    Json,
}

impl Cli {
    pub fn run(&self) -> anyhow::Result<()> {
        if let Some(shell) = self.completions {
            return Self::completions(shell);
        }
        if let Some(format) = self.init {
            return Self::init(format);
        }
        self.generate()
    }

    fn generate(&self) -> anyhow::Result<()> {
        let (config, base) = self.read_config()?;

        let spec = match (&self.openapi, &config.source) {
            (Some(path), _) => path.clone(),
            (None, Some(source)) => base.join(source),
            (None, None) => {
                bail!("OpenAPI spec is required: pass it or set `source` in the config")
            }
        };
        if config.rules.is_empty() && config.files.is_empty() {
            bail!("config emits nothing: add `rules` or a file entry");
        }

        let ir = oapi::Oapi::from_file(&spec)
            .context("Failed to parse OpenAPI spec")?
            .lower()
            .context("Transforming OpenAPI spec to IR failed")?;

        if !config.rules.is_empty() {
            let ts = render(&ir, config.rules, &config.formats)?;
            match &self.output {
                Some(output) => std::fs::write(output, ts)
                    .with_context(|| format!("Failed to write {}", output.display()))?,
                None => print!("{ts}"),
            }
        }

        for (path, rules) in config.files {
            let path = base.join(path);
            let ts = render(&ir, rules, &config.formats)
                .with_context(|| format!("Failed to generate {}", path.display()))?;
            std::fs::write(&path, ts)
                .with_context(|| format!("Failed to write {}", path.display()))?;
        }
        Ok(())
    }

    /// The config, and the directory its relative paths resolve against.
    fn read_config(&self) -> anyhow::Result<(Config, PathBuf)> {
        let Some(path) = &self.config else {
            return Ok((Config::default(), PathBuf::new()));
        };
        let config = Config::from_file(path).context("Failed to read config")?;
        let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
        Ok((config, base))
    }

    fn init(format: Format) -> anyhow::Result<()> {
        match format {
            Format::Yaml => print!("{DEFAULT_CONFIG_YAML}"),
            Format::Json => {
                let value: serde_json::Value = serde_yaml::from_str(DEFAULT_CONFIG_YAML)
                    .context("The default config is not valid YAML")?;
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
        }
        Ok(())
    }

    fn completions(shell: Shell) -> anyhow::Result<()> {
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut io::stdout());
        Ok(())
    }
}

/// One output module: the rules run against `ir` in order, then rendered
/// together so they can reference each other's declarations.
fn render(
    ir: &ast::Doc,
    rules: Vec<Rule>,
    formats: &IndexMap<String, String>,
) -> anyhow::Result<String> {
    let mut builder = Builder::new(ir);
    builder.formats(formats.clone());
    for rule in rules {
        builder.with(rule).context("Failed to apply rule")?;
    }
    builder.render().context("Failed to render")
}
