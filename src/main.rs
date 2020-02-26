use brchd::args::Args;
use brchd::errors::*;
use brchd::http;
use env_logger::Env;
use std::io::stdout;
use structopt::StructOpt;

async fn run() -> Result<()> {
    let args = Args::from_args();
    debug!("{:#?}", args);

    if args.daemon {
        todo!();
    } else if args.http_daemon {
        http::run(&args).await?;
    } else if let Some(shell) = args.gen_completions {
        Args::clap().gen_completions_to("brchd", shell, &mut stdout());
    } else if !args.paths.is_empty() {
        println!("upload: {:?}", args.paths);
    } else {
        println!("empty args");
    }

    Ok(())
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default()
        .default_filter_or("actix_server=info,actix_web=info"));

    if let Err(err) = run().await {
        eprintln!("Error: {}", err);
        for cause in err.iter_chain().skip(1) {
            eprintln!("Because: {}", cause);
        }
        std::process::exit(1);
    }

    Ok(())
}
