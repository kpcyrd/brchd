use crate::errors::*;
use nom::{
    IResult,
    bytes::complete::{tag, take},
    number::complete::{be_u16},
    combinator::map_opt,
};
use serde::{Serialize, Deserialize};
use sodiumoxide::crypto::box_::{self, Nonce, PublicKey, SecretKey, NONCEBYTES, PUBLICKEYBYTES};
use sodiumoxide::crypto::secretstream::{self, Key, Stream, Pull};
use std::convert::TryFrom;

const MAGIC: &[u8] = b"\x00#BRCHD\x00";
const MAGIC_SIZE: usize = 8;
pub const HEADER_INTRO_LEN: usize = MAGIC_SIZE + NONCEBYTES + PUBLICKEYBYTES + 2;

type Intro = (Nonce, PublicKey, u16);
pub type RawHeader = (Nonce, PublicKey, Vec<u8>);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Header {
    pub key: Vec<u8>,
    pub next_header: Vec<u8>,
    pub name: Option<String>,
}

impl Header {
    pub fn encrypt(&self, pubkey: &PublicKey) -> Result<Vec<u8>> {
        let header = serde_json::to_vec(self)?;

        // TODO: we should pad the header because of the file name

        let nonce = box_::gen_nonce();
        let (pk, sk) = box_::gen_keypair();

        let header = box_::seal(&header, &nonce, pubkey, &sk);

        let len = u16::try_from(header.len())
            .context("File encryption header is too large")?;

        let mut out = Vec::from(MAGIC);
        out.extend(&nonce[..]);
        out.extend(&pk[..]);
        out.extend(&len.to_be_bytes());
        out.extend(&header);

        Ok(out)
    }

    pub fn open_stream_pull(&self) -> Result<Stream<Pull>> {
        let next_header = secretstream::Header::from_slice(&self.next_header)
            .ok_or_else(|| format_err!("Invalid secretstream header"))?;
        let key = Key::from_slice(&self.key)
            .ok_or_else(|| format_err!("Invalid secretstream key"))?;

        Stream::init_pull(&next_header, &key)
            .map_err(|_| format_err!("Failed to open file decryption stream"))
    }
}

pub fn decrypt(nonce: &Nonce, pk: &PublicKey, data: &[u8], sk: &SecretKey) -> Result<Header> {
    let header = box_::open(data, nonce, pk, sk)
        .map_err(|_| format_err!("Failed to decrypt header"))?;
    let header = serde_json::from_slice(&header)?;
    Ok(header)
}

pub fn decrypt_slice(input: &[u8], sk: &SecretKey) -> Result<Header> {
    let (input, (
        nonce,
        pk,
        len,
    )) = intro(input)
        .map_err(|e| format_err!("Failed to parse encryption header intro: {}", e))?;

    if input.len() != len as usize {
        bail!("Failed to read encryption header body");
    }

    decrypt(&nonce, &pk, input, sk)
}

pub fn parse_intro(input: &[u8]) -> Result<Intro> {
    intro(input)
        .map(|(_, x)| x)
        .map_err(|e| format_err!("Failed to parse encryption header intro: {}", e))
}

fn intro(input: &[u8]) -> IResult<&[u8], Intro> {
    let (input, _) = tag(MAGIC)(input)?;
    let (input, nonce) = nonce(input)?;
    let (input, pk) = pubkey(input)?;
    let (input, len) = be_u16(input)?;
    Ok((input, (nonce, pk, len)))
}

fn nonce(input: &[u8]) -> IResult<&[u8], Nonce> {
    map_opt(
        take(NONCEBYTES),
        Nonce::from_slice
    )(input)
}

fn pubkey(input: &[u8]) -> IResult<&[u8], PublicKey> {
    map_opt(
        take(PUBLICKEYBYTES),
        PublicKey::from_slice
    )(input)
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
        let h1 = Header {
            key: vec![1,2,3,4],
            next_header: vec![5,6,7,8],
            name: Some("ohai.txt".to_string()),
        };
        let header = h1.encrypt(&sk.public_key()).expect("encrypt");
        println!("header: {:?}", header);
        let h2 = decrypt_slice(&header, &sk).expect("decrypt");
        assert_eq!(h1, h2);
    }
}
