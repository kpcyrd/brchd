use actix_multipart::Multipart;
use actix_web::{middleware, web, App, Error, HttpResponse, HttpServer};
use brch::args::Args;
use brch::errors::*;
use env_logger::Env;
use futures::StreamExt;
use std::io::stdout;
use std::io::Write;
use structopt::StructOpt;

async fn save_file(mut payload: Multipart) -> std::result::Result<HttpResponse, Error> {
    // iterate over multipart stream
    while let Some(item) = payload.next().await {
        let mut field = item?;
        let content_type = field.content_disposition().unwrap();
        let filename = content_type.get_filename().unwrap(); // TODO
        let filepath = format!("./tmp/{}", filename);
        // File::create is blocking operation, use threadpool
        let mut f = web::block(|| std::fs::File::create(filepath))
            .await
            .unwrap(); // TODO
        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap(); // TODO
            // filesystem operations are blocking, we have to use threadpool
            f = web::block(move || f.write_all(&data).map(|_| f)).await?;
        }
    }
    // Ok(HttpResponse::Ok().into())
    Ok(HttpResponse::Ok().body("done."))
}

fn index() -> HttpResponse {
    let html = r#"<html>
    <head><title>Upload File</title></head>
    <body>
    <form target="/" method="post" enctype="multipart/form-data">
    <input type="file" multiple name="file">
    <input type="submit" value="Submit">
    </form>
    </body>
    </html>"#;

    HttpResponse::Ok().body(html)
}

async fn run() -> Result<()> {
    let args = Args::from_args();
    println!("{:#?}", args);

    if !args.paths.is_empty() {
        println!("upload: {:?}", args.paths);
    } else if args.daemon {
        std::fs::create_dir_all("./tmp")?;

        let ip = "0.0.0.0:3000";

        HttpServer::new(|| {
                App::new().wrap(middleware::Logger::default()).service(
                    web::resource("/")
                        .route(web::get().to(index))
                        .route(web::post().to(save_file)),
                )
            })
            .bind(ip)?
            .run()
            .await?;
    } else if let Some(shell) = args.gen_completions {
        Args::clap().gen_completions_to("brch", shell, &mut stdout());
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
