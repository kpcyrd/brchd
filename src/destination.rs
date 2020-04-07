use actix_web::{web, error, Error as ResponseError};
use crate::errors::*;
use crate::pathspec::UploadContext;
use crate::temp;
use futures::{Stream, StreamExt};
use humansize::{FileSize, file_size_opts};
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::fs::{self, File, OpenOptions};

const MAX_DEST_OPEN_ATTEMPTS: u8 = 12;

pub fn get_filename(p: &Path) -> Result<(String, String)> {
    let mut i = p.iter().peekable();

    let mut pb = PathBuf::new();
    while let Some(x) = i.next() {
        match x.to_str() {
            Some("/") => (), // skip this
            Some("..") => bail!("Directory traversal detected"),
            Some(p) => {
                pb.push(&p);
                if i.peek().is_none() {
                    return Ok((
                        // we've ensured that the path is valid utf-8, unwrap is fine
                        pb.to_str().unwrap().to_string(),
                        p.to_string(),
                    ));
                }
            },
            None => bail!("Filename is invalid utf8"),
        }
    }

    bail!("Path is an empty string")
}

pub struct UploadHandle {
    pub dest_path: PathBuf,
    pub temp_path: PathBuf,
    pub f: File,
}

pub fn open_upload_dest(ctx: UploadContext) -> Result<UploadHandle> {
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

pub fn save_sync<R: Read>(stream: &mut R, ctx: UploadContext) -> Result<()> {
    let mut upload = open_upload_dest(ctx)?;
    info!("writing file into {:?}", upload.temp_path);

    let size = io::copy(stream, &mut upload.f)?;

    let size = size.file_size(file_size_opts::CONVENTIONAL)
        .map_err(|e| format_err!("{}", e))?;

    info!("moving file {:?} -> {:?} ({})", upload.temp_path, upload.dest_path, size);
    fs::rename(upload.temp_path, upload.dest_path)
        .context("Failed to move temp file to final destination")
        .map_err(Error::from)?;

    Ok(())
}
