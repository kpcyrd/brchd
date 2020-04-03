use crate::crypto::header;
use crate::errors::*;
use sodiumoxide::crypto::box_::{SecretKey, PublicKey};
use sodiumoxide::crypto::secretstream::{self, Header, Stream, Push, Pull, Tag, ABYTES};
use std::io::prelude::*;

pub const CHUNK_SIZE: usize = 4096;

pub struct CryptoReader<T> {
    r: T,
    header: header::Header,
    stream: Stream<Pull>,
}

impl<T: Read> CryptoReader<T> {
    pub fn init(mut r: T, seckey: &SecretKey, pubkey: Option<&PublicKey>) -> Result<Option<CryptoReader<T>>> {
        let (nonce, pk, header) = match CryptoReader::read_header(&mut r) {
            Ok(h) => h,
            Err(_) => return Ok(None),
        };

        // in strict mode, ensure the pubkey is the one we expect
        if let Some(pubkey) = pubkey {
            if *pubkey != pk {
                bail!("Header is signed by untrusted publickey");
            }
        }

        let header = header::decrypt(&nonce, &pk, &header, seckey)?;

        let next_header = CryptoReader::read_next_header(&mut r)?;
        let stream = header.open_stream_pull(&next_header)?;

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

    fn read_next_header(r: &mut T) -> Result<Header> {
        let mut buf = [0u8; secretstream::HEADERBYTES];
        r.read_exact(&mut buf)
            .context("Failed to read next header")?;

        secretstream::Header::from_slice(&buf)
            .ok_or_else(|| format_err!("Invalid secretstream header"))
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
    pub fn init(mut w: T, pubkey: &PublicKey, seckey: Option<&SecretKey>) -> Result<CryptoWriter<T>> {
        let key = secretstream::gen_key();

        let header = header::Header {
            key: key.0.to_vec(),
            name: None,
        };
        let header = header.encrypt(pubkey, seckey)?;
        w.write_all(&header)?;

        let (stream, header) = Stream::init_push(&key).unwrap();
        w.write_all(&header.0)?;

        Ok(CryptoWriter {
            w,
            stream,
        })
    }

    #[inline]
    pub fn inner(&self) -> &T {
        &self.w
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.w
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
    use sodiumoxide::crypto::box_;
    use super::*;

    fn sec() -> SecretKey {
        SecretKey::from_slice(&[
            75, 34, 106, 31, 123, 150, 128, 79, 208, 89, 61, 66, 53, 35, 62, 111,
            41, 78, 178, 55, 187, 47, 244, 155, 61, 206, 49, 130, 219, 28, 104, 5,
        ]).unwrap()
    }

    const FILE: &[u8] = &[
        0x00, 0x23, 0x42, 0x52, 0x43, 0x48, 0x44, 0x00, 0xf5, 0x6c, 0x3b, 0x42,
        0x1f, 0xd2, 0x81, 0x7c, 0x4e, 0x4b, 0x43, 0x1f, 0x9d, 0x16, 0x02, 0x0d,
        0x8d, 0xd4, 0x61, 0xba, 0xe2, 0x55, 0xf8, 0x83, 0xbd, 0xea, 0x08, 0xc8,
        0x52, 0x31, 0x13, 0xfe, 0xa6, 0x6f, 0x31, 0x9d, 0x20, 0xad, 0xfc, 0x7f,
        0xec, 0xb2, 0xa4, 0x31, 0x7a, 0xe6, 0x6e, 0xb1, 0xe7, 0x50, 0x06, 0xae,
        0x95, 0x14, 0x31, 0x4f, 0x00, 0x44, 0x0d, 0x6a, 0x6f, 0x76, 0xfb, 0xd2,
        0xee, 0x60, 0x37, 0xe3, 0xeb, 0x67, 0xec, 0x37, 0x4c, 0x9c, 0x3e, 0xc9,
        0x7c, 0xb8, 0xbf, 0x0e, 0xa8, 0x4c, 0x09, 0x3f, 0xf1, 0x30, 0xba, 0xc7,
        0xc1, 0xde, 0x20, 0xc1, 0xa2, 0x32, 0x50, 0xed, 0x60, 0xaa, 0x4d, 0x1a,
        0xa1, 0xc3, 0x52, 0x1e, 0x6e, 0xdb, 0x1f, 0xfc, 0x6e, 0x06, 0x27, 0xf1,
        0x73, 0x69, 0xf3, 0x6d, 0xb7, 0x38, 0x47, 0xcc, 0xad, 0x62, 0xc6, 0x2e,
        0x01, 0x13, 0x1c, 0xd0, 0xa1, 0x25, 0xbf, 0xad, 0xbf, 0x0e, 0xa8, 0x4d,
        0x5c, 0x31, 0xf8, 0x67, 0xa1, 0x3a, 0x1d, 0x7d, 0xac, 0x79, 0x20, 0xc7,
        0xf5, 0xa6, 0xcc, 0xcd, 0xa4, 0xc3, 0xd0, 0x9b, 0x1f, 0xc0, 0xce, 0x67,
        0x7f, 0xda, 0xfb, 0xe4, 0xcb, 0x53, 0xa7, 0x33, 0x9a, 0x49, 0xdb, 0x8f,
        0x66,
    ];

    #[test]
    fn roundtrip() {
        let sk = sec();
        let pk = sk.public_key();

        let mut file = Vec::new();
        let mut w = CryptoWriter::init(&mut file, &pk, None).unwrap();
        w.push(b"ohai!\n", true).unwrap();

        let mut cur = std::io::Cursor::new(&file);
        let mut r = CryptoReader::init(&mut cur, &sk, None).unwrap().unwrap();
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
        let mut r = CryptoReader::init(&mut cur, &sk, None).unwrap().unwrap();
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
        let mut r = CryptoReader::init(&mut cur, &sk, None).unwrap().unwrap();
        assert!(r.is_not_finalized());

        let mut buf = Vec::new();
        let r = r.pull(&mut buf);
        assert!(r.is_err());
    }

    #[test]
    fn empty_file() {
        let sk = sec();
        let mut cur = std::io::Cursor::new(&[]);
        let r = CryptoReader::init(&mut cur, &sk, None).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn missing_header_body() {
        let sk = sec();
        let mut cur = std::io::Cursor::new(&FILE[..header::HEADER_INTRO_LEN]);
        let r = CryptoReader::init(&mut cur, &sk, None).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn missing_next_header() {
        let sk = sec();

        let intro = header::parse_intro(&FILE[..header::HEADER_INTRO_LEN]).unwrap();
        let len = header::HEADER_INTRO_LEN + intro.2 as usize;
        let mut cur = std::io::Cursor::new(&FILE[..len]);
        let r = CryptoReader::init(&mut cur, &sk, None);
        assert!(r.is_err());
    }

    #[test]
    fn missing_stream() {
        let sk = sec();

        let intro = header::parse_intro(&FILE[..header::HEADER_INTRO_LEN]).unwrap();
        let len = header::HEADER_INTRO_LEN + intro.2 as usize + secretstream::HEADERBYTES;
        let mut cur = std::io::Cursor::new(&FILE[..len]);
        let mut r = CryptoReader::init(&mut cur, &sk, None).unwrap().unwrap();

        let mut buf = Vec::new();
        let r = r.pull(&mut buf);
        assert!(r.is_err());
    }

    #[test]
    fn init_only_writes_header() {
        let sk = sec();
        let pk = sk.public_key();

        let mut file = Vec::new();
        let _w = CryptoWriter::init(&mut file, &pk, None).unwrap();
        assert_eq!(file.len(), 158);
    }

    #[test]
    fn authenticated_crypto_roundtrip() {
        let (our_pk, our_sk) = box_::gen_keypair();
        let (their_pk, their_sk) = box_::gen_keypair();

        let mut file = Vec::new();
        let mut w = CryptoWriter::init(&mut file, &their_pk, Some(&our_sk)).unwrap();
        w.push(b"ohai!\n", true).unwrap();

        let mut cur = std::io::Cursor::new(&file);
        let mut r = CryptoReader::init(&mut cur, &their_sk, Some(&our_pk)).unwrap().unwrap();
        assert!(r.is_not_finalized());

        let mut buf = Vec::new();
        r.pull(&mut buf).unwrap();
        assert!(!r.is_not_finalized());

        assert_eq!(&buf, b"ohai!\n");
    }

    #[test]
    fn authenticated_crypto_encrypt() {
        let (our_pk, our_sk) = box_::gen_keypair();
        let (their_pk, _their_sk) = box_::gen_keypair();

        let mut file = Vec::new();
        let mut w = CryptoWriter::init(&mut file, &their_pk, Some(&our_sk)).unwrap();
        w.push(b"ohai!\n", true).unwrap();

        assert_eq!(&file[32..64], &our_pk.0)
    }

    #[test]
    fn authenticated_crypto_decrypt_strict_key() {
        let sk = sec();
        let (pk, _) = box_::gen_keypair();

        let mut cur = std::io::Cursor::new(FILE);
        let r = CryptoReader::init(&mut cur, &sk, Some(&pk));
        assert!(r.is_err());
    }
}
