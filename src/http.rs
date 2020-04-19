use actix_multipart::Multipart;
use actix_multipart::Field;
use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse};
use actix_web::{web, App, Error as ResponseError, HttpResponse, HttpServer};
use crate::args::Args;
use crate::config::UploadConfig;
use crate::destination::{self, save_async};
use crate::errors::*;
use crate::pathspec::UploadContext;
use futures::{Future, StreamExt};
use futures::future::{ok, Ready};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

fn filename(field: &Field) -> Option<String> {
    let content_type = field.content_disposition()?;
    let path = content_type.get_filename()?;
    Some(path.to_string())
}

async fn post_file(req: web::HttpRequest, config: web::Data<Arc<UploadConfig>>, mut payload: Multipart) -> std::result::Result<HttpResponse, ResponseError> {
    let remote_addr = remote_addr(&req.peer_addr());
    let remote_sock = remote_sock(&req.peer_addr());

    // iterate over multipart stream
    while let Some(item) = payload.next().await {
        let field = item?;

        if let Some(path) = filename(&field) {
            save_async(field, UploadContext::new(
                config.destination.clone(),
                config.path_format.clone(),
                Some(remote_addr.clone()),
                &path,
                None,
            )?, remote_sock.clone()).await?;
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

    destination::save_async(payload, UploadContext::new(
        config.destination.clone(),
        config.path_format.clone(),
        Some(remote_addr),
        &filename,
        None,
    )?, remote_sock).await?;

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
    type Future = Pin<Box<dyn Future<Output = std::result::Result<Self::Response, ResponseError>>>>;

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
