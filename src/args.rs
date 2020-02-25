use std::path::PathBuf;
use structopt::StructOpt;
use structopt::clap::{AppSettings, Shell};

#[derive(Debug, StructOpt)]
#[structopt(global_settings = &[AppSettings::ColoredHelp])]
pub struct Args {
    pub paths: Vec<PathBuf>,
    #[structopt(short, long)]
    pub daemon: bool,
    #[structopt(long, possible_values=&Shell::variants())]
    pub gen_completions: Option<Shell>,
}
