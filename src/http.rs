use actix_multipart::Multipart;
use actix_multipart::Field;
use actix_web::{middleware, web, App, Error as ResponseError, HttpResponse, HttpServer};
use crate::args::Args;
use crate::config::UploadConfig;
use crate::errors::*;
use chrono::Utc;
use futures::StreamExt;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::io::Write;
use std::path::{Path, PathBuf};
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

pub struct UploadHandle {
    dest_path: PathBuf,
    temp_path: String,
    f: File,
}

fn open_upload_dest(dest: String, filename: String) -> std::io::Result<UploadHandle> {
    loop {
        let dt = Utc::now();
        let today = dt.format("%Y-%m-%d").to_string();

        let id = random_id();

        let path = format!("{}/{}/{}-{}", dest, today, id, filename);
        let dest_path = PathBuf::from(path);
        let parent = dest_path.parent().expect("Destination path has no parent");
        fs::create_dir_all(parent)?;

        if let Ok(_f) = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dest_path)
        {
            let temp_path = format!("{}/{}/.{}-{}.part", dest, today, id, filename);
            let f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)?;

            return Ok(UploadHandle {
                dest_path,
                temp_path,
                f,
            });
        }
    }
}

async fn recv_all(mut field: Field, mut f: File) -> std::result::Result<(), ResponseError> {
    // Field in turn is stream of *Bytes* object
    while let Some(chunk) = field.next().await {
        let data = chunk?;
        // filesystem operations are blocking, we have to use threadpool
        f = web::block(move || f.write_all(&data).map(|_| f)).await?;
    }
    Ok(())
}

async fn save_file(config: web::Data<Arc<UploadConfig>>, mut payload: Multipart) -> std::result::Result<HttpResponse, ResponseError> {
    // iterate over multipart stream
    while let Some(item) = payload.next().await {
        let field = item?;

        if let Some(filename) = filename(&field)? {
            // filesystem operations are blocking, we have to use threadpool
            let upload_dest = config.destination.clone();
            let upload = web::block(|| open_upload_dest(upload_dest, filename))
                .await?;

            recv_all(field, upload.f).await?;

            fs::rename(upload.temp_path, upload.dest_path)
                .context("Failed to move temp file to final destination")
                .map_err(Error::from)?;
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

#[actix_rt::main]
pub async fn run(args: Args) -> Result<()> {
    let config = Arc::new(UploadConfig::load(&args)?);

    std::fs::create_dir_all(&config.destination)?;

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
