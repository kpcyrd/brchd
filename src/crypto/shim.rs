use crate::args::Args;
use crate::errors::*;

pub type PublicKey = ();
pub type SecretKey = ();

pub mod upload {
    use std::fs::File;
    use std::io;
    use super::*;

    pub struct EncryptedUpload {
    }

    impl EncryptedUpload {
        pub fn new(_: File, _: &PublicKey, _: Option<&SecretKey>) -> Result<EncryptedUpload> {
            unimplemented!()
        }

        pub fn total_with_overhead(&self, _: u64) -> u64 {
            unimplemented!()
        }
    }

    impl io::Read for EncryptedUpload {
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
