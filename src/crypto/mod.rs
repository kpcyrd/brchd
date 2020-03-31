use crate::args::Args;
use sodiumoxide::crypto::box_;
use crate::config::{EncryptConfig, DecryptConfig};
use crate::crypto::stream::{CryptoReader, CryptoWriter};
use crate::errors::*;
use crate::temp;
use humansize::{FileSize, file_size_opts};
use std::fs::{self, File, OpenOptions};
use std::io::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

pub mod header;
pub mod stream;

fn walk<F>(paths: &[String], f: F) -> Result<()>
where
    F: Fn(&Path) -> Result<()>
{
    for path in paths {
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Err(e) = f(path) {
                    error!("error: {}", e);
                }
            }
        }
    }
    Ok(())
}

pub fn run_encrypt(args: Args) -> Result<()> {
    let config = EncryptConfig::load(&args)?;

    walk(&args.paths, |path| {
        let (_, temp_path) = temp::partial_path(&path)
            .context("Failed to get partial path")?;

        info!("encrypting {:?}", path);
        let mut r = File::open(path)?;
        let w = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;

        let mut w = CryptoWriter::init(w, &config.pubkey)?;

        let mut size = 0;
        let mut buf = [0u8; stream::CHUNK_SIZE];
        loop {
            let n = r.read(&mut buf)?;
            if n == 0 {
                break;
            }
            w.push(&buf[..n], n != stream::CHUNK_SIZE)?;
            size += n;
        }

        let size = size.file_size(file_size_opts::CONVENTIONAL)
            .map_err(|e| format_err!("{}", e))?;

        debug!("finishing encryption {:?} -> {:?} ({})", temp_path, path, size);
        fs::rename(temp_path, path)
            .context("Failed to move temp file to final destination")?;

        Ok(())
    })
}

pub fn run_decrypt(args: Args) -> Result<()> {
    let config = DecryptConfig::load(&args)?;

    walk(&args.paths, |path| {
        debug!("peeking into {:?}", path);
        let file = File::open(path)?;
        if let Some(mut r) = CryptoReader::init(file, &config.seckey)? {
            info!("decrypting {:?}", path);

            let (_, temp_path) = temp::partial_path(&path)
                .context("Failed to get partial path")?;
            let mut w = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)?;

            let mut size = 0;
            let mut buf = Vec::with_capacity(stream::CHUNK_SIZE);
            while r.is_not_finalized() {
                buf.clear();
                r.pull(&mut buf)?;
                w.write_all(&buf)?;
                size += buf.len();
            }

            let size = size.file_size(file_size_opts::CONVENTIONAL)
                .map_err(|e| format_err!("{}", e))?;

            debug!("finishing decryption {:?} -> {:?} ({})", temp_path, path, size);
            fs::rename(temp_path, path)
                .context("Failed to move temp file to final destination")?;
        }
        Ok(())
    })
}

pub fn run_keygen(_args: Args) -> Result<()> {
    let (pk, sk) = box_::gen_keypair();
    let pk = base64::encode(&pk);
    let sk = base64::encode(&sk);
    println!("[crypto]");
    println!("#pubkey = {:?}", pk);
    println!("seckey = {:?}", sk);
    Ok(())
}
