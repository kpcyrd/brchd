use crate::crypto::stream::{self, CryptoWriter};
use crate::errors::*;
use sodiumoxide::crypto::box_::{PublicKey, SecretKey};
use sodiumoxide::crypto::secretstream;
use std::io;
use std::io::prelude::*;

pub struct EncryptedUpload<R> {
    inner: R,
    header: Vec<u8>,
    stream: CryptoWriter<Vec<u8>>,
    eof: bool,
}

impl<R> EncryptedUpload<R> {
    pub fn new(inner: R, pubkey: &PublicKey, seckey: Option<&SecretKey>) -> Result<EncryptedUpload<R>> {
        let buf = Vec::new();
        let stream = CryptoWriter::init(buf, pubkey, seckey)?;
        let header = stream.inner().clone();

        Ok(EncryptedUpload {
            inner,
            header,
            stream,
            eof: false,
        })
    }

    pub fn total_with_overhead(&self, total: u64) -> u64 {
        let carry = (total % stream::CHUNK_SIZE as u64) + secretstream::ABYTES as u64;
        let total = (total / stream::CHUNK_SIZE as u64) * (stream::CHUNK_SIZE + secretstream::ABYTES) as u64;
        self.header.len() as u64 + total + carry
    }
}

impl<R: Read> Read for EncryptedUpload<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        while !self.eof && self.stream.inner().len() < out.len()  {
            let mut buf = [0u8; stream::CHUNK_SIZE];
            let n = self.inner.read(&mut buf)?;

            if n != stream::CHUNK_SIZE {
                self.eof = true;
            }

            self.stream.push(&buf[..n], self.eof)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }

        let buf = self.stream.inner_mut();
        let len = std::cmp::min(buf.len(), out.len());
        // TODO: this is probably very inefficient
        for (i, b) in buf.drain(..len).enumerate() {
            out[i] = b;
        }

        Ok(len)
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
