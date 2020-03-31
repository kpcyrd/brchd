use crate::crypto::header;
use crate::errors::*;
use sodiumoxide::crypto::box_::{SecretKey, PublicKey};
use sodiumoxide::crypto::secretstream::{self, Stream, Push, Pull, Tag, ABYTES};
use std::io::prelude::*;

pub const CHUNK_SIZE: usize = 4096;

pub struct CryptoReader<T> {
    r: T,
    header: header::Header,
    stream: Stream<Pull>,
}

impl<T: Read> CryptoReader<T> {
    pub fn init(mut r: T, sk: &SecretKey) -> Result<Option<CryptoReader<T>>> {
        let (nonce, pk, header) = match CryptoReader::read_header(&mut r) {
            Ok(h) => h,
            Err(_) => return Ok(None),
        };

        let header = header::decrypt(&nonce, &pk, &header, sk)?;
        let stream = header.open_stream_pull()?;

        Ok(Some(CryptoReader {
            r,
            header,
            stream,
        }))
    }

    fn read_header(r: &mut T) -> Result<header::RawHeader> {
        let mut intro = [0u8; header::HEADER_INTRO_LEN];
        r.read_exact(&mut intro)
            .context("Failed to read encryption header intro")?;

        let (nonce, pk, len) = header::parse_intro(&intro)?;

        let mut header = vec![0u8; len as usize];
        r.read_exact(&mut header)
            .context("Failed to read encryption header body")?;

        Ok((nonce, pk, header))
    }

    pub fn filename(&self) -> Option<&String> {
        self.header.name.as_ref()
    }

    pub fn pull(&mut self, out: &mut Vec<u8>) -> Result<()> {
        let mut buf = [0u8; CHUNK_SIZE + ABYTES];
        let n = self.r.read(&mut buf)?;
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

pub struct CryptoWriter<T> {
    w: T,
    stream: Stream<Push>,
}

impl<T: Write> CryptoWriter<T> {
    pub fn init(mut w: T, pubkey: &PublicKey) -> Result<CryptoWriter<T>> {
        let key = secretstream::gen_key();
        let (stream, header) = Stream::init_push(&key).unwrap();

        let header = header::Header {
            key: key.0.to_vec(),
            next_header: header.0.to_vec(),
            name: None,
        };
        let header = header.encrypt(pubkey)?;
        w.write_all(&header)?;

        Ok(CryptoWriter {
            w,
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
        self.w.write_all(&c)?;
        debug!("wrote {} bytes to file", c.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sec() -> SecretKey {
        SecretKey::from_slice(&[
            75, 34, 106, 31, 123, 150, 128, 79, 208, 89, 61, 66, 53, 35, 62, 111,
            41, 78, 178, 55, 187, 47, 244, 155, 61, 206, 49, 130, 219, 28, 104, 5,
        ]).unwrap()
    }

    #[test]
    fn roundtrip() {
        let sk = sec();
        let pk = sk.public_key();

        let mut file = Vec::new();
        let mut w = CryptoWriter::init(&mut file, &pk).unwrap();
        w.push(b"ohai!\n", true).unwrap();

        let mut cur = std::io::Cursor::new(&file);
        let mut r = CryptoReader::init(&mut cur, &sk).unwrap().unwrap();
        assert!(r.is_not_finalized());

        let mut buf = Vec::new();
        r.pull(&mut buf).unwrap();
        assert!(!r.is_not_finalized());

        assert_eq!(&buf, b"ohai!\n");
    }
}
