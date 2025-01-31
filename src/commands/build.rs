use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{arg, value_parser, ArgMatches, Command};

use crate::{core::BuildContext, create_http_client, model::{Server, Lockfile, Network}};

pub fn cli() -> Command {
    Command::new("build")
        .about("Build using server.toml configuration")
        .arg(
            arg!(-o --output [FILE] "The output directory for the server")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(arg!(--skip [stages] "Skip some stages").value_delimiter(','))
        .arg(arg!(--force "Don't skip downloading already downloaded jars"))
}

pub async fn run(matches: &ArgMatches) -> Result<BuildContext> {
    let server = Server::load().context("Failed to load server.toml")?;
    let network = Network::load()?;
    let http_client = create_http_client()?;

    let default_output = server.path.join("server");
    let output_dir = matches
        .get_one::<PathBuf>("output")
        .unwrap_or(&default_output)
        .clone();

    let lockfile = Lockfile::get_lockfile(&output_dir)?;

    let force = matches.get_flag("force");

    let skip_stages = matches
        .get_many::<String>("skip")
        .map(|o| o.cloned().collect::<Vec<String>>())
        .unwrap_or(vec![]);

    std::fs::create_dir_all(&output_dir).context("Failed to create output directory")?;

    let mut ctx = BuildContext {
        server,
        network,
        http_client,
        force,
        skip_stages,
        lockfile,
        output_dir,
        ..Default::default()
    };

    ctx.build_all().await?;

    Ok(ctx)
}
