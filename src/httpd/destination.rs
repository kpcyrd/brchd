use actix_web::{web, error, Error as ResponseError};
use crate::destination::open_upload_dest;
use crate::errors::*;
use crate::pathspec::UploadContext;
use futures::{Stream, StreamExt};
use humansize::{FileSize, file_size_opts};
use std::io::prelude::*;
use std::fs::{self, File};

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

pub async fn save_async<S, E>(stream: S, ctx: UploadContext, remote_sock: String) -> std::result::Result<(), ResponseError>
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

    let temp_path = upload.temp_path;
    let dest_path = upload.dest_path;
    info!("{} moving upload {:?} -> {:?} ({})", remote_sock, temp_path, dest_path, size);
    web::block(|| fs::rename(temp_path, dest_path)
        .context("Failed to move temp file to final destination")
        .map_err(Error::from)
    ).await?;

    Ok(())
}
