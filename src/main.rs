use std::path::PathBuf;

use anyhow::{Result, bail};
use facet::Facet;
use figue::{self as args, Driver};

use opi::{Config, openapi};

/// Generate TypeScript types from an OpenAPI spec.
#[derive(Debug, Facet)]
struct Cli {
    /// OpenAPI spec to read (`.json`, `.yaml` or `.yml`).
    #[facet(args::positional, default)]
    spec: Option<PathBuf>,

    /// File to write the generated TypeScript to. Prints to stdout if omitted.
    #[facet(args::positional, default)]
    out: Option<PathBuf>,

    /// Generator config (`tsapi.toml`). Uses the built-in defaults if omitted.
    #[facet(args::named, args::short = 'c', args::label = "FILE", default)]
    config: Option<PathBuf>,

    /// Print the default config as commented TOML and exit: a starting point
    /// for `tsapi.toml`.
    #[facet(args::named, default)]
    print_config: bool,

    /// Standard `--help`, `--version` and completion flags.
    #[facet(flatten)]
    builtins: args::FigueBuiltins,
}

fn main() -> Result<()> {
    let cli = parse_args();

    if cli.print_config {
        print!("{}", opi::DEFAULT_CONFIG_TOML);
        return Ok(());
    }
    let config = match &cli.config {
        Some(path) => Config::from_file(path)?,
        None => Config::default(),
    };
    let Some(spec) = &cli.spec else {
        bail!("missing <SPEC>; run with --help for usage");
    };
    let spec = openapi::Spec::from_file(spec)?;
    let code = opi::generate(&spec, &config)?;

    match &cli.out {
        Some(path) => std::fs::write(path, code)?,
        None => print!("{code}"),
    }
    Ok(())
}

/// Handles `--help` / `--version` / parse errors itself (printing and exiting).
fn parse_args() -> Cli {
    let config = figue::builder::<Cli>()
        .expect("Cli is a valid figue schema")
        .cli(|cli| cli.args(std::env::args().skip(1)))
        .help(|help| {
            help.program_name(env!("CARGO_PKG_NAME"))
                .version(env!("CARGO_PKG_VERSION"))
        })
        .build();
    Driver::new(config).run().unwrap()
}
