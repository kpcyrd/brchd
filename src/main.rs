use brchd::args::Args;
use brchd::config::ClientConfig;
use brchd::daemon;
use brchd::errors::*;
use brchd::http;
use brchd::ipc::IpcClient;
use brchd::queue;
use brchd::status::StatusWriter;
use env_logger::Env;
use std::io::stdout;
use structopt::StructOpt;

fn log_filter(verbose: u8) -> &'static str {
    match verbose {
        0 => "actix_server=info,actix_web=info,brchd=warn",
        1 => "actix_server=info,actix_web=info,brchd=info",
        2 => "actix_server=info,actix_web=info,brchd=debug",
        3 => "info,brchd=debug",
        _ => "debug",
    }
}

fn run() -> Result<()> {
    let args = Args::from_args();

    env_logger::init_from_env(Env::default()
        .default_filter_or(log_filter(args.verbose)));

    if args.daemon {
        daemon::run(&args)?;
    } else if args.http_daemon {
        http::run(args)?;
    } else if args.wait {
        let config = ClientConfig::load(&args)?;
        let mut client = IpcClient::connect(&config.socket)?;
        client.subscribe()?;
        while let Some(status) = client.read_status()? {
            if status.queue == 0 && status.idle_workers == status.total_workers {
                break;
            }
        }
    } else if let Some(shell) = args.gen_completions {
        Args::clap().gen_completions_to("brchd", shell, &mut stdout());
    } else if !args.paths.is_empty() {
        queue::run_add(args)?;
    } else {
        // TODO: add --once option
        let config = ClientConfig::load(&args)?;
        let mut client = IpcClient::connect(&config.socket)?;
        client.subscribe()?;
        let mut w = StatusWriter::new();
        while let Some(status) = client.read_status()? {
            w.write(status)?;
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        for cause in err.iter_chain().skip(1) {
            eprintln!("Because: {}", cause);
        }
        std::process::exit(1);
    }
}
