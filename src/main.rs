use std::path::PathBuf;
use std::process::exit;

use anyhow::{Result, bail};

use opi::{Config, Spec};

const HELP: &str = "\
Generate TypeScript types from an OpenAPI spec.

Usage: opi [OPTIONS] <SPEC> [OUT]

Arguments:
  <SPEC>  OpenAPI spec to read (.json, .yaml or .yml)
  [OUT]   File to write to; stdout if omitted

Options:
  -c, --config <FILE>  Generator config (.yaml, .yml or .json); the
                       built-in defaults if omitted
      --print-config   Print the default config as commented YAML and exit
      --emit-ir        Write the operations and schemas as JSON (with rendered
                       TS types) instead of TypeScript
      --explain        Print what the config selects instead of the code
  -h, --help           Print help
  -V, --version        Print version
";

#[derive(Debug, Default)]
struct Cli {
    spec: Option<PathBuf>,
    out: Option<PathBuf>,
    config: Option<PathBuf>,
    print_config: bool,
    emit_ir: bool,
    explain: bool,
}

fn main() -> Result<()> {
    let cli = match parse_args() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("error: {e}\n\nRun with --help for usage.");
            exit(2);
        }
    };

    if cli.print_config {
        print!("{}", opi::DEFAULT_CONFIG_YAML);
        return Ok(());
    }
    let config = match &cli.config {
        Some(path) => Config::from_file(path)?,
        None => Config::default(),
    };
    let Some(spec) = &cli.spec else {
        bail!("missing <SPEC>; run with --help for usage");
    };
    let spec = Spec::from_file(spec)?;
    let output = if cli.explain {
        opi::explain(&spec, &config)?
    } else if cli.emit_ir {
        opi::generate_ir(&spec, &config)?
    } else {
        opi::generate(&spec, &config)?
    };

    match &cli.out {
        Some(path) => std::fs::write(path, output)?,
        None => print!("{output}"),
    }
    Ok(())
}

/// Handles `--help` / `--version` itself (printing and exiting).
fn parse_args() -> Result<Cli, lexopt::Error> {
    use lexopt::prelude::*;

    let mut cli = Cli::default();
    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('c') | Long("config") => cli.config = Some(parser.value()?.into()),
            Long("print-config") => cli.print_config = true,
            Long("emit-ir") => cli.emit_ir = true,
            Long("explain") => cli.explain = true,
            Short('h') | Long("help") => {
                print!("{HELP}");
                exit(0);
            }
            Short('V') | Long("version") => {
                println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                exit(0);
            }
            Value(value) if cli.spec.is_none() => cli.spec = Some(value.into()),
            Value(value) if cli.out.is_none() => cli.out = Some(value.into()),
            _ => return Err(arg.unexpected()),
        }
    }
    if cli.emit_ir && cli.explain {
        return Err("--emit-ir and --explain can't be used together".into());
    }
    Ok(cli)
}
