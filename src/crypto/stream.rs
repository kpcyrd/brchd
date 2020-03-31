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
            .map_err(|_| format_err!("Failed to decrypt secretstream chunk"))?;
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

    const FILE: &[u8] = &[
        0x00, 0x23, 0x42, 0x52, 0x43, 0x48, 0x44, 0x00, 0xb9, 0x52, 0x85, 0xff,
        0xac, 0x7d, 0x5f, 0xba, 0x37, 0x93, 0x29, 0x02, 0xe6, 0x04, 0xab, 0x7a,
        0xb5, 0x0e, 0x3e, 0xad, 0x94, 0x34, 0x79, 0xd6, 0xcf, 0x70, 0xd6, 0x30,
        0xd1, 0xea, 0x2d, 0x2b, 0x84, 0x42, 0x14, 0xc5, 0x7f, 0xf9, 0xa5, 0xd4,
        0xf0, 0x72, 0x8e, 0x44, 0xa8, 0xe6, 0xd7, 0x3f, 0x2f, 0xd1, 0x93, 0x0c,
        0xa3, 0x4d, 0xfc, 0x59, 0x00, 0x6b, 0x33, 0x05, 0xbb, 0x31, 0x09, 0x87,
        0x49, 0xab, 0x9f, 0x8b, 0x08, 0xe1, 0xe0, 0x6d, 0x06, 0xda, 0x14, 0xbb,
        0x28, 0x43, 0x16, 0xcb, 0x92, 0x96, 0x7b, 0x72, 0x60, 0xb6, 0xfd, 0x5e,
        0xe9, 0x87, 0x31, 0x84, 0x08, 0x01, 0xc3, 0xed, 0x86, 0x21, 0x23, 0xc6,
        0x4c, 0x88, 0xab, 0x4d, 0x58, 0xae, 0xd4, 0x60, 0x6f, 0xaf, 0x2c, 0xcc,
        0x13, 0x28, 0x1c, 0x04, 0x3f, 0x71, 0xf4, 0xe0, 0x2f, 0xf9, 0x9a, 0x7d,
        0x50, 0xc0, 0x11, 0xfc, 0x97, 0xd5, 0x7c, 0x9c, 0xc9, 0xea, 0xfa, 0x57,
        0x0a, 0x48, 0xa7, 0x67, 0x68, 0x9a, 0xd8, 0xf1, 0xec, 0xc0, 0x84, 0x2d,
        0x5b, 0x0e, 0x28, 0x82, 0x1d, 0x62, 0xbf, 0x5b, 0x56, 0x8b, 0xf3, 0x5b,
        0x7c, 0xc5, 0x13, 0x7d, 0x64, 0x17, 0xd5, 0x1d, 0x01, 0xd8, 0x60, 0x63,
        0x66, 0x26, 0x39, 0xa6, 0x6b, 0xf5, 0xa5, 0xdd, 0x07, 0x40, 0xc6, 0x62,
        0xae, 0xfa, 0x50, 0xa0,
    ];

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

    #[test]
    fn decrypt() {
        let sk = sec();

        let mut cur = std::io::Cursor::new(FILE);
        let mut r = CryptoReader::init(&mut cur, &sk).unwrap().unwrap();
        assert!(r.is_not_finalized());

        let mut buf = Vec::new();
        r.pull(&mut buf).unwrap();
        assert!(!r.is_not_finalized());

        assert_eq!(&buf, b"ohai!\n");
    }

    #[test]
    fn trailing_data() {
        let sk = sec();

        let mut buf = Vec::from(FILE);
        buf.push(123);

        let mut cur = std::io::Cursor::new(&buf);
        let mut r = CryptoReader::init(&mut cur, &sk).unwrap().unwrap();
        assert!(r.is_not_finalized());

        let mut buf = Vec::new();
        let r = r.pull(&mut buf);
        assert!(r.is_err());
    }

    #[test]
    fn empty_file() {
        let sk = sec();
        let mut cur = std::io::Cursor::new(&[]);
        let r = CryptoReader::init(&mut cur, &sk).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn missing_header_body() {
        let sk = sec();
        let mut cur = std::io::Cursor::new(&FILE[..header::HEADER_INTRO_LEN]);
        let r = CryptoReader::init(&mut cur, &sk).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn missing_stream() {
        let sk = sec();

        let intro = header::parse_intro(&FILE[..header::HEADER_INTRO_LEN]).unwrap();
        let mut cur = std::io::Cursor::new(&FILE[..header::HEADER_INTRO_LEN + intro.2 as usize]);
        let mut r = CryptoReader::init(&mut cur, &sk).unwrap().unwrap();

        let mut buf = Vec::new();
        let r = r.pull(&mut buf);
        assert!(r.is_err());
    }
}
