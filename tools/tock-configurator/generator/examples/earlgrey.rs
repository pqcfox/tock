use clap::{command, Parser};
use std::error::Error;
use std::path::PathBuf;
use tock_generator::{Lowrisc, TockMain};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct PathArgs {
    #[arg(long)]
    config: PathBuf,

    #[arg(long)]
    output: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = PathArgs::parse();
    let tock_main = TockMain::from_json(Lowrisc::default(), args.config)?;
    tock_main.write_to_file(args.output)?;

    Ok(())
}
