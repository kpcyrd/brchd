use crate::crypto::header;
use crate::errors::*;
use sodiumoxide::crypto::box_::{SecretKey, PublicKey};
use sodiumoxide::crypto::secretstream::{self, Stream, Push, Pull, Tag, ABYTES};
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

pub const CHUNK_SIZE: usize = 4096;

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

    pub fn pull(&mut self, out: &mut Vec<u8>) -> Result<()> {
        let mut buf = [0u8; CHUNK_SIZE + ABYTES];
        let n = self.f.read(&mut buf)?;
        if n == 0 {
            bail!("Unexpected end of file");
        }
        debug!("read {} bytes from file", n);

        let tag = self.stream.pull_to_vec(&buf[..n], None, out)
            .map_err(|_| format_err!("Failed to open secretstream"))?;
        debug!("decrypted {} bytes, tag={:?}", out.len(), tag);

        Ok(())
    }

    pub fn is_not_finalized(&self) -> bool {
        self.stream.is_not_finalized()
    }
}

pub struct CryptoWriter {
    f: File,
    stream: Stream<Push>,
}

impl CryptoWriter {
    pub fn init(mut f: File, pubkey: &PublicKey) -> Result<CryptoWriter> {
        let key = secretstream::gen_key();
        let (stream, header) = Stream::init_push(&key).unwrap();

        let header = header::Header {
            key: key.0.to_vec(),
            next_header: header.0.to_vec(),
            name: None,
        };
        let header = header.encrypt(pubkey)?;
        f.write_all(&header)?;

        Ok(CryptoWriter {
            f,
            stream,
        })
    }

    pub fn push(&mut self, buf: &[u8], is_final: bool) -> Result<()> {
        let tag = if !is_final {
            Tag::Message
        } else {
            Tag::Final
        };

        debug!("encrypting {} bytes, tag={:?}", buf.len(), tag);
        let c = self.stream.push(buf, None, tag)
            .map_err(|_| format_err!("Failed to write to secretstream"))?;
        self.f.write_all(&c)?;
        debug!("wrote {} bytes to file", c.len());

        Ok(())
    }
}
