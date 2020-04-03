use crate::crypto::stream::{self, CryptoWriter};
use crate::errors::*;
use sodiumoxide::crypto::box_::{PublicKey, SecretKey};
use sodiumoxide::crypto::secretstream;
use std::io;
use std::io::prelude::*;

pub struct EncryptedUpload<R> {
    inner: R,
    header_len: u64,
    stream: CryptoWriter,
    buf: Vec<u8>,
    cursor: usize,
    eof: bool,
}

impl<R: Read> EncryptedUpload<R> {
    pub fn new(inner: R, pubkey: &PublicKey, seckey: Option<&SecretKey>) -> Result<EncryptedUpload<R>> {
        let mut buf = Vec::new();
        let stream = CryptoWriter::init(&mut buf, pubkey, seckey)?;
        let header_len = buf.len() as u64;

        Ok(EncryptedUpload {
            inner,
            header_len,
            stream,
            buf,
            cursor: 0,
            eof: false,
        })
    }

    pub fn total_with_overhead(&self, total: u64) -> u64 {
        let carry = (total % stream::CHUNK_SIZE as u64) + secretstream::ABYTES as u64;
        let total = (total / stream::CHUNK_SIZE as u64) * (stream::CHUNK_SIZE + secretstream::ABYTES) as u64;
        self.header_len + total + carry
    }

    fn fill_bytes(&mut self) -> io::Result<()> {
        // refill buffer
        let mut buf = [0u8; stream::CHUNK_SIZE];
        let n = self.inner.read(&mut buf)?;

        if n != stream::CHUNK_SIZE {
            self.eof = true;
        }

        // reset our cursor
        self.cursor = 0;
        self.stream.push(&buf[..n], self.eof, &mut self.buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(())
    }

    fn cursor(&self) -> &[u8] {
        &self.buf[self.cursor..]
    }
}

impl<R: Read> Read for EncryptedUpload<R> {
    fn read(&mut self, mut out: &mut [u8]) -> io::Result<usize> {
        let mut n = 0;

        while (!self.eof || self.cursor().len() > 0) && out.len() > 0 {
            // check if we need to refill our buffer
            if !self.eof && self.cursor().len() == 0 {
                trace!("buffering encrypted bytes");
                self.fill_bytes()?;
            }

            // copy from our buffer into the read buffer
            let buf = self.cursor();
            let len = std::cmp::min(buf.len(), out.len());
            let len = Read::read(&mut &buf[..len], &mut out[..len])?;

            out = &mut out[len..];
            self.cursor += len;
            n += len;
        }

        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::stream::CryptoReader;
    use sodiumoxide::crypto::box_;
    use std::io::Cursor;
    use super::*;

    #[test]
    fn verify_calculated_total_short_file() {
        let len = 16;
        let bytes = sodiumoxide::randombytes::randombytes(len);

        let r = Cursor::new(bytes);
        let (pk, _sk) = box_::gen_keypair();
        let mut upload = EncryptedUpload::new(r, &pk, None).unwrap();
        let estimated = upload.total_with_overhead(len as u64);

        let mut encrypted = Vec::new();
        upload.read_to_end(&mut encrypted).unwrap();
        assert_eq!(encrypted.len() as u64, estimated);
    }

    #[test]
    fn verify_calculated_total_long_file() {
        let len = 1024 * 1024 * 16;
        let bytes = sodiumoxide::randombytes::randombytes(len);

        let r = Cursor::new(bytes);
        let (pk, _sk) = box_::gen_keypair();
        let mut upload = EncryptedUpload::new(r, &pk, None).unwrap();
        let estimated = upload.total_with_overhead(len as u64);

        let mut encrypted = Vec::new();
        upload.read_to_end(&mut encrypted).unwrap();
        assert_eq!(encrypted.len() as u64, estimated);
    }

    #[test]
    fn encrypt_into_buf_decrypt_short() {
        let bytes = sodiumoxide::randombytes::randombytes(16);

        // encrypt
        let r = Cursor::new(&bytes);
        let (pk, sk) = box_::gen_keypair();
        let mut upload = EncryptedUpload::new(r, &pk, None).unwrap();

        let mut encrypted = Vec::new();
        upload.read_to_end(&mut encrypted).unwrap();

        // decrypt
        let r = Cursor::new(encrypted);
        let mut r = CryptoReader::init(r, &sk, None).unwrap().unwrap();

        let mut decrypted: Vec<u8> = Vec::new();
        let mut buf = Vec::new();
        while r.is_not_finalized() {
            r.pull(&mut buf).unwrap();
            decrypted.extend(&buf);
        }

        // compare
        assert_eq!(decrypted, bytes);
    }

    #[test]
    fn encrypt_into_buf_decrypt_long() {
        let bytes = sodiumoxide::randombytes::randombytes(1024 * 1024 * 16);

        // encrypt
        let r = Cursor::new(&bytes);
        let (pk, sk) = box_::gen_keypair();
        let mut upload = EncryptedUpload::new(r, &pk, None).unwrap();

        let mut encrypted = Vec::new();
        upload.read_to_end(&mut encrypted).unwrap();

        // decrypt
        let r = Cursor::new(encrypted);
        let mut r = CryptoReader::init(r, &sk, None).unwrap().unwrap();

        let mut decrypted: Vec<u8> = Vec::new();
        let mut buf = Vec::new();
        while r.is_not_finalized() {
            r.pull(&mut buf).unwrap();
            decrypted.extend(&buf);
        }

        // compare
        assert_eq!(decrypted, bytes);
    }
}
