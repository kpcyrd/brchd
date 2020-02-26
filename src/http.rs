use actix_multipart::Multipart;
use actix_multipart::Field;
use actix_web::{middleware, web, App, Error, HttpResponse, HttpServer};
use crate::args::Args;
use crate::config::Config;
use crate::errors::*;
use chrono::Utc;
use futures::StreamExt;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::io::Write;
use std::path::Path;
use std::fs::{self, File, OpenOptions};
use std::sync::Arc;

fn filename(field: &Field) -> Result<Option<String>> {
    let content_type = match field.content_disposition() {
        Some(x) => x,
        _ => return Ok(None),
    };
    let filename = match content_type.get_filename() {
        Some(x) => x,
        _ => return Ok(None),
    };

    // TODO: consider just writing a secure_join
    let path = Path::new(filename);
    for x in path.iter() {
        match x.to_str() {
            Some("/") => bail!("Filename is absolute path"),
            Some("..") => bail!("Directory traversal detected"),
            None => bail!("Filename is invalid utf8"),
            _ => (),
        }
    }

    Ok(Some(filename.to_string()))
}

fn random_id() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(4)
        .collect()
}

fn open_upload_dest(dest: String, filename: String) -> std::io::Result<File> {
    loop {
        let dt = Utc::now();
        let today = dt.format("%Y-%m-%d").to_string();

        let id = random_id();

        let path = format!("{}/{}/{}-{}", dest, today, id, filename);
        let filepath = Path::new(&path);
        let parent = filepath.parent().expect("Destination path has no parent");
        fs::create_dir_all(parent)?;

        if let Ok(f) = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(filepath)
        {
            return Ok(f);
        }
    }
}

async fn save_file(config: web::Data<Arc<Config>>, mut payload: Multipart) -> std::result::Result<HttpResponse, Error> {
    // iterate over multipart stream
    while let Some(item) = payload.next().await {
        let mut field = item?;

        if let Some(filename) = filename(&field)? {
            // filesystem operations are blocking, we have to use threadpool
            let upload_dest = config.upload_dest.clone();
            let mut f = web::block(|| open_upload_dest(upload_dest, filename))
                .await?;

            // Field in turn is stream of *Bytes* object
            while let Some(chunk) = field.next().await {
                let data = chunk?; // TODO: this was unwrap
                // filesystem operations are blocking, we have to use threadpool
                f = web::block(move || f.write_all(&data).map(|_| f)).await?;
            }
        }
    }
    Ok(HttpResponse::Ok().body("done.\n"))
}

fn index() -> HttpResponse {
    let html = r#"<html>
    <head><title>Upload File</title></head>
    <body>
    <form action="/" method="post" enctype="multipart/form-data">
    <input type="file" multiple name="file">
    <input type="submit" value="Submit">
    </form>
    </body>
    </html>"#;

    HttpResponse::Ok().body(html)
}

pub async fn run(args: &Args) -> Result<()> {
    let config = Arc::new(Config::load(&args)?);

    std::fs::create_dir_all(&config.upload_dest)?;

    let app_data = config.clone();
    HttpServer::new(move || {
            App::new()
                .data(app_data.clone())
                .wrap(middleware::Logger::default()).service(web::resource("/")
                    .route(web::get().to(index))
                    .route(web::post().to(save_file)),
            )
        })
        .bind(config.bind_addr)?
        .run()
        .await?;
    Ok(())
}
