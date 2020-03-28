use actix_multipart::Multipart;
use actix_multipart::Field;
use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, dev::ConnectionInfo};
use actix_web::{web, App, Error as ResponseError, HttpResponse, HttpServer};
use crate::args::Args;
use crate::config::UploadConfig;
use crate::errors::*;
use chrono::Utc;
use futures::{Future, StreamExt};
use futures::future::{ok, Ready};
use humansize::{FileSize, file_size_opts};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::fs::{self, File, OpenOptions};
use std::sync::Arc;
use std::task::{Context, Poll};

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

async fn recv_all(mut field: Field, mut f: File) -> std::result::Result<usize, ResponseError> {
    let mut n = 0;
    // Field in turn is stream of *Bytes* object
    while let Some(chunk) = field.next().await {
        let data = chunk?;
        n += data.len();
        // filesystem operations are blocking, we have to use threadpool
        f = web::block(move || f.write_all(&data).map(|_| f)).await?;
    }
    Ok(n)
}

async fn save_file(req: web::HttpRequest, config: web::Data<Arc<UploadConfig>>, mut payload: Multipart) -> std::result::Result<HttpResponse, ResponseError> {
    let remote = remote(&req.connection_info());

    // iterate over multipart stream
    while let Some(item) = payload.next().await {
        let field = item?;

        if let Some(filename) = filename(&field)? {
            // filesystem operations are blocking, we have to use threadpool
            let upload_dest = config.destination.clone();
            let upload = web::block(|| open_upload_dest(upload_dest, filename))
                .await?;
            info!("{} writing upload into {:?}", remote, upload.temp_path);

            let size = recv_all(field, upload.f).await?;

            let size = size.file_size(file_size_opts::CONVENTIONAL)
                .map_err(|e| format_err!("{}", e))?;

            info!("{} moving upload {:?} -> {:?} ({})", remote, upload.temp_path, upload.dest_path, size);
            fs::rename(upload.temp_path, upload.dest_path)
                .context("Failed to move temp file to final destination")
                .map_err(Error::from)?;
        }
    }
    Ok(HttpResponse::Ok().body("done.\n"))
}

fn index() -> HttpResponse {
    let html = r#"<!DOCTYPE html>
<html>
    <head><title>Upload File</title></head>
    <body>
        <form action="/" method="post" enctype="multipart/form-data">
            <input type="file" multiple name="file">
            <input type="submit" value="Submit">
        </form>
    </body>
</html>
"#;

    HttpResponse::Ok().body(html)
}

pub struct Logger;

impl<S, B> Transform<S> for Logger
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = ResponseError>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = ResponseError;
    type InitError = ();
    type Transform = LoggerMiddleware<S>;
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoggerMiddleware { service })
    }
}

pub struct LoggerMiddleware<S> {
    service: S,
}

impl<S, B> Service for LoggerMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = ResponseError>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = ResponseError;
    type Future = Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let log = LogResponse::new(&req);
        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            log.write(&res);
            Ok(res)
        })
    }
}

fn remote(ci: &ConnectionInfo) -> String {
    ci.remote()
        .map(|r| r.to_string())
        .unwrap_or_else(|| "-".to_string())
}

struct LogResponse {
    remote: String,
    request_line: String,
    user_agent: String,
}

impl LogResponse {
    fn new(req: &ServiceRequest) -> LogResponse {
        let remote = remote(&req.connection_info());

        let request_line = if req.query_string().is_empty() {
            format!(
                "{} {} {:?}",
                req.method(),
                req.path(),
                req.version()
            )
        } else {
            format!(
                "{} {}?{} {:?}",
                req.method(),
                req.path(),
                req.query_string(),
                req.version()
            )
        };

        let user_agent = if let Some(val) = req.headers().get("User-Agent") {
            if let Ok(s) = val.to_str() {
                s
            } else {
                "-"
            }
        } else {
            "-"
        }.to_string();

        LogResponse {
            remote,
            request_line,
            user_agent,
        }
    }

    fn write<B>(self, res: &ServiceResponse<B>) {
        let status_code = res.response().head().status.as_u16();
        info!("{} {:?} {} {:?}",
            self.remote,
            self.request_line,
            status_code,
            self.user_agent
        )
    }
}

#[actix_rt::main]
pub async fn run(args: Args) -> Result<()> {
    let config = Arc::new(UploadConfig::load(&args)?);

    std::fs::create_dir_all(&config.destination)?;

    info!("starting brchd http daemon on {}", config.bind_addr);
    let app_data = config.clone();
    HttpServer::new(move || {
            App::new()
                .data(app_data.clone())
                .wrap(Logger)
                .service(web::resource("/")
                    .route(web::get().to(index))
                    .route(web::post().to(save_file)),
            )
        })
        .bind(config.bind_addr)?
        .run()
        .await?;
    Ok(())
}
