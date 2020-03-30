use crate::crypto::header;
use crate::errors::*;
use sodiumoxide::crypto::box_::SecretKey;
// use sodiumoxide::crypto::secretstream::{Header, Key, Stream, Pull, Tag};
use sodiumoxide::crypto::secretstream::{Stream, Pull, Tag, ABYTES};
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

const CHUNK_SIZE: usize = 4096;

pub struct CryptoReader {
    f: File,
    header: header::Header,
    stream: Stream<Pull>,
}

impl CryptoReader {
    pub fn open(path: &Path, sk: &SecretKey) -> Result<Option<CryptoReader>> {
        let mut f = File::open(path)?;

        let (nonce, pk, header) = match CryptoReader::read_header(&mut f) {
            Ok(h) => h,
            Err(_) => return Ok(None),
        };

        let header = header::decrypt(&nonce, &pk, &header, sk)?;
        let stream = header.open_stream_pull()?;

        Ok(Some(CryptoReader {
            f,
            header,
            stream,
        }))
    }

    fn read_header(f: &mut File) -> Result<header::RawHeader> {
        let mut intro = [0u8; header::HEADER_INTRO_LEN];
        f.read_exact(&mut intro)
            .context("Failed to read encryption header intro")?;

        let (nonce, pk, len) = header::parse_intro(&intro)?;

        let mut header = vec![0u8; len as usize];
        f.read_exact(&mut header)
            .context("Failed to read encryption header body")?;

        Ok((nonce, pk, header))
    }

    pub fn filename(&self) -> Option<&String> {
        self.header.name.as_ref()
    }

    pub fn pull(&mut self, out: &mut Vec<u8>) -> Result<bool> {
        let mut buf = [0u8; CHUNK_SIZE + ABYTES];
        let n = self.f.read(&mut buf)?;
        if n == 0 {
            bail!("Unexpected end of file");
        }

        let tag = self.stream.pull_to_vec(&buf[..n], None, out)
            .map_err(|_| format_err!("Failed to open secretstream"))?;

        match tag {
            Tag::Message => Ok(true),
            Tag::Final => Ok(false),
            _ => bail!("Unexpected stream tag"),
        }
    }
}
