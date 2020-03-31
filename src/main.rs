use brchd::args::Args;
use brchd::config::ClientConfig;
use brchd::crypto;
use brchd::daemon;
use brchd::errors::*;
use brchd::http;
use brchd::ipc::IpcClient;
use brchd::queue;
use brchd::status::StatusWriter;
use env_logger::Env;
use std::io::stdout;
use structopt::StructOpt;

fn log_filter(args: &Args) -> &'static str {
    let mut verbose = args.verbose;

    // make sure verbose is always >= 1 for request logging
    if args.http_daemon && verbose == 0 {
        verbose = 1;
    }

    match verbose {
        0 => "brchd=warn",
        1 => "brchd=info",
        2 => "brchd=debug",
        3 => "info,brchd=debug",
        4 => "debug",
        _ => "debug,brchd=trace",
    }
}

fn run() -> Result<()> {
    let args = Args::from_args();

    env_logger::init_from_env(Env::default()
        .default_filter_or(log_filter(&args)));

    if args.daemon {
        daemon::run(&args)?;
    } else if args.http_daemon {
        http::run(args)?;
    } else if args.encrypt {
        crypto::run_encrypt(args)?;
    } else if args.decrypt {
        crypto::run_decrypt(args)?;
    } else if args.keygen {
        crypto::run_keygen(args)?;
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
