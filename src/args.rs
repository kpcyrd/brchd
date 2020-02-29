use structopt::StructOpt;
use structopt::clap::{AppSettings, Shell};

#[derive(Debug, StructOpt)]
#[structopt(global_settings = &[AppSettings::ColoredHelp])]
pub struct Args {
    pub paths: Vec<String>,
    #[structopt(short, long, group="action")]
    pub daemon: bool,
    #[structopt(short="H", long, group="action")]
    pub http_daemon: bool,
    #[structopt(long, possible_values=&Shell::variants(), group="action")]
    pub gen_completions: Option<Shell>,
    #[structopt(short="p", long)]
    pub upload_dest: Option<String>,
}
