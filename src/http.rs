use actix_multipart::Multipart;
use actix_multipart::Field;
use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse};
use actix_web::{web, App, error, Error as ResponseError, HttpResponse, HttpServer};
use crate::args::Args;
use crate::config::UploadConfig;
use crate::errors::*;
use crate::pathspec::UploadContext;
use crate::temp;
use futures::{Future, Stream, StreamExt};
use futures::future::{ok, Ready};
use humansize::{FileSize, file_size_opts};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::fs::{self, File, OpenOptions};
use std::sync::Arc;
use std::task::{Context, Poll};

const MAX_DEST_OPEN_ATTEMPTS: u8 = 12;

fn filename(field: &Field) -> Result<Option<(String, String)>> {
    let content_type = match field.content_disposition() {
        Some(x) => x,
        _ => return Ok(None),
    };
    let path = match content_type.get_filename() {
        Some(x) => x,
        _ => return Ok(None),
    };

    let p = Path::new(path);
    let mut i = p.iter().peekable();

    let mut pb = PathBuf::new();
    while let Some(x) = i.next() {
        match x.to_str() {
            Some("/") => (), // skip this
            Some("..") => bail!("Directory traversal detected"),
            Some(p) => {
                pb.push(&p);
                if i.peek().is_none() {
                    return Ok(Some((
                        // we've ensured that the path is valid utf-8, unwrap is fine
                        pb.to_str().unwrap().to_string(),
                        p.to_string(),
                    )));
                }
            },
            None => bail!("Filename is invalid utf8"),
        }
    }

    bail!("Path is an empty string")
}

pub struct UploadHandle {
    dest_path: PathBuf,
    temp_path: PathBuf,
    f: File,
}

fn open_upload_dest(ctx: UploadContext) -> Result<UploadHandle> {
    for _ in 0..MAX_DEST_OPEN_ATTEMPTS {
        let (path, deterministic) = ctx.generate()?;

        let dest = Path::new(&ctx.destination);
        let dest_path = dest.join(path);

        let (parent, temp_path) = temp::partial_path(&dest_path)
            .context("Failed to get partial path")?;
        fs::create_dir_all(parent)?;

        if let Ok(_f) = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dest_path)
        {
            let f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)?;

            return Ok(UploadHandle {
                dest_path,
                temp_path,
                f,
            });
        } else if deterministic {
            warn!("refusing to overwrite {:?}", dest_path);
            bail!("Target file already exists")
        }
    }

    bail!("Failed to find new filename to upload to")
}

async fn recv_all<S, E>(mut stream: S, mut f: File) -> std::result::Result<usize, ResponseError>
where
    S: Stream<Item=std::result::Result<web::Bytes, E>> + Unpin,
    E: 'static + error::ResponseError,
{
    let mut n = 0;

    while let Some(chunk) = stream.next().await {
        let data = chunk?;
        n += data.len();
        // filesystem operations are blocking, we have to use threadpool
        f = web::block(move || f.write_all(&data).map(|_| f)).await?;
    }

    Ok(n)
}

async fn save<S, E>(stream: S, ctx: UploadContext, remote_sock: String) -> std::result::Result<(), ResponseError>
where
    S: Stream<Item=std::result::Result<web::Bytes, E>> + Unpin,
    E: 'static + error::ResponseError,
{
    // filesystem operations are blocking, we have to use threadpool
    let upload = web::block(|| open_upload_dest(ctx))
        .await?;
    info!("{} writing upload into {:?}", remote_sock, upload.temp_path);

    let size = recv_all(stream, upload.f).await?;

    let size = size.file_size(file_size_opts::CONVENTIONAL)
        .map_err(|e| format_err!("{}", e))?;

    info!("{} moving upload {:?} -> {:?} ({})", remote_sock, upload.temp_path, upload.dest_path, size);
    fs::rename(upload.temp_path, upload.dest_path)
        .context("Failed to move temp file to final destination")
        .map_err(Error::from)?;

    Ok(())
}

async fn post_file(req: web::HttpRequest, config: web::Data<Arc<UploadConfig>>, mut payload: Multipart) -> std::result::Result<HttpResponse, ResponseError> {
    let remote_addr = remote_addr(&req.peer_addr());
    let remote_sock = remote_sock(&req.peer_addr());

    // iterate over multipart stream
    while let Some(item) = payload.next().await {
        let field = item?;

        if let Some((path, filename)) = filename(&field)? {
            save(field, UploadContext::new(
                config.destination.clone(),
                config.path_format.clone(),
                remote_addr.clone(),
                filename,
                path,
                None,
            ), remote_sock.clone()).await?;
        }
    }

    Ok(HttpResponse::Ok().body("done.\n"))
}

async fn put_file(req: web::HttpRequest, config: web::Data<Arc<UploadConfig>>, payload: web::Payload) -> std::result::Result<HttpResponse, ResponseError> {
    let remote_addr = remote_addr(&req.peer_addr());
    let remote_sock = remote_sock(&req.peer_addr());

    let mut filename = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .collect::<String>();
    filename.push_str(".dat");

    save(payload, UploadContext::new(
        config.destination.clone(),
        config.path_format.clone(),
        remote_addr,
        filename.clone(),
        filename,
        None,
    ), remote_sock).await?;

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

fn remote_addr(sa: &Option<SocketAddr>) -> String {
    sa
        .map(|r| r.ip().to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn remote_sock(sa: &Option<SocketAddr>) -> String {
    sa
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
        let remote = remote_sock(&req.peer_addr());

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
                .service(web::resource("/*")
                    .route(web::get().to(index))
                    .route(web::post().to(post_file))
                    .route(web::put().to(put_file))
            )
        })
        .bind(config.bind_addr)?
        .run()
        .await?;
    Ok(())
}
