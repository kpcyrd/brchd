use crate::args::Args;
use crate::errors::*;
use std::io::Read;
use std::marker::PhantomData;

pub type PublicKey = ();
pub type SecretKey = ();

pub mod upload {
    use std::io;
    use super::*;

    pub struct EncryptedUpload<R> {
        phantom: PhantomData<R>,
    }

    impl<R: Read> EncryptedUpload<R> {
        pub fn new(_: R, _: &PublicKey, _: Option<&SecretKey>) -> Result<EncryptedUpload<R>> {
            unimplemented!()
        }

        pub fn total_with_overhead(&self, _: u64) -> u64 {
            unimplemented!()
        }
    }

    impl<R> io::Read for EncryptedUpload<R> {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            unimplemented!()
        }
    }
}

pub fn decode_pubkey(_key: &str) -> Result<PublicKey> {
    unimplemented!()
}

pub fn decode_seckey(_key: &str) -> Result<PublicKey> {
    unimplemented!()
}

pub fn run_encrypt(_: Args) -> Result<()> {
    unimplemented!()
}

pub fn run_decrypt(_: Args) -> Result<()> {
    unimplemented!()
}

pub fn run_keygen(_: Args) -> Result<()> {
    unimplemented!()
}
