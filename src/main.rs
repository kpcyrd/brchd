use brchd::args::Args;
use brchd::daemon;
use brchd::errors::*;
use brchd::http;
use brchd::ipc::IpcClient;
use brchd::spider;
use brchd::status::StatusWriter;
use brchd::walkdir;
use env_logger::Env;
use reqwest::Client;
use std::io::stdout;
use std::time::Duration;
use structopt::StructOpt;

async fn run() -> Result<()> {
    let args = Args::from_args();
    debug!("{:#?}", args);

    if args.daemon {
        daemon::run(&args)?;
    } else if args.http_daemon {
        http::run(&args).await?;
    } else if args.wait {
        let mut client = IpcClient::connect(&args.socket()?)?;
        client.subscribe()?;
        while let Some(status) = client.read_status()? {
            if status.queue == 0 && status.idle_workers == status.total_workers {
                break;
            }
        }
    } else if let Some(shell) = args.gen_completions {
        Args::clap().gen_completions_to("brchd", shell, &mut stdout());
    } else if !args.paths.is_empty() {
        let mut client = IpcClient::connect(&args.socket()?)?;

        let http = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(60))
            .build()?;

        for path in &args.paths {
            if path.starts_with("https://") || path.starts_with("https://") {
                spider::queue(&mut client, &http, path).await?;
            } else {
                walkdir::queue(&mut client, path)?;
            }
        }
    } else {
        // TODO: add --once option
        let mut client = IpcClient::connect(&args.socket()?)?;
        client.subscribe()?;
        let mut w = StatusWriter::new();
        while let Some(status) = client.read_status()? {
            w.write(status)?;
        }
    }

    Ok(())
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default()
        .default_filter_or("actix_server=info,actix_web=info,brchd=warn"));

    if let Err(err) = run().await {
        eprintln!("Error: {}", err);
        for cause in err.iter_chain().skip(1) {
            eprintln!("Because: {}", cause);
        }
        std::process::exit(1);
    }

    Ok(())
}
