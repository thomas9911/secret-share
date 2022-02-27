use aes_gcm_siv::aead::rand_core::{OsRng, RngCore};
use aes_gcm_siv::aead::{Aead, NewAead};
use aes_gcm_siv::{Aes256GcmSiv, Nonce};
use derive_more::{Display, From};
use flate2::bufread::{GzDecoder, GzEncoder};
use flate2::Compression;
use sha2::digest::consts::U32;
use sha2::digest::generic_array::GenericArray;
use sha2::Digest;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;

type AeadKey = aes_gcm_siv::Key<<Aes256GcmSiv as NewAead>::KeySize>;

#[derive(Debug, Display, From, derive_more::Error)]
pub enum Error {
    Aead(aes_gcm_siv::aead::Error),
    Io(std::io::Error),
}

#[derive(Debug)]
pub struct Key {
    inner_key: AeadKey,
}

impl<T: AsRef<str>> From<T> for Key {
    fn from(input: T) -> Key {
        let hash = sha2::Sha256::digest(input.as_ref());
        let inner_key = aes_gcm_siv::Key::from_slice(&hash);

        Key {
            inner_key: *inner_key,
        }
    }
}

impl AsRef<AeadKey> for Key {
    fn as_ref(&self) -> &AeadKey {
        &self.inner_key
    }
}

pub fn sha256<T: AsRef<str>>(input: T) -> GenericArray<u8, U32> {
    sha2::Sha256::digest(input.as_ref())
}

pub fn encode<R: Read, W: Write>(in_reader: R, out_writer: W, key: &Key) -> Result<(), Error> {
    let nonce = Nonce::from(random_bytes());

    let mut encoder = GzEncoder::new(BufReader::new(in_reader), Compression::default());

    let mut buffer = Vec::new();
    encoder.read_to_end(&mut buffer)?;

    let cipher = Aes256GcmSiv::new(key.as_ref());
    let ciphertext = cipher.encrypt(&nonce, &buffer[..])?;

    store(out_writer, &ciphertext, &nonce)?;
    Ok(())
}

pub fn decode<R: Read, W: Write>(in_reader: R, mut out_writer: W, key: &Key) -> Result<(), Error> {
    let mut new_ciphertext = Vec::new();
    let cipher = Aes256GcmSiv::new(key.as_ref());

    let new_nonce = load(in_reader, &mut new_ciphertext)?;
    let plaintext = cipher.decrypt(&new_nonce, new_ciphertext.as_ref())?;

    let mut gz = GzDecoder::new(&plaintext[..]);
    io::copy(&mut gz, &mut out_writer)?;
    Ok(())
}

fn store<W: Write>(mut writer: W, bytes: &[u8], nonce: &Nonce) -> Result<(), Error> {
    writer.write_all(nonce)?;
    writer.write_all(bytes)?;
    Ok(())
}

fn load<R: Read>(reader: R, buffer: &mut Vec<u8>) -> Result<Nonce, Error> {
    let mut bufreader = BufReader::new(reader);
    let mut bytes = [0u8; 12];
    bufreader.read_exact(&mut bytes)?;
    let nonce = Nonce::from(bytes);
    bufreader.read_to_end(buffer)?;
    Ok(nonce)
}

pub fn random_bytes() -> [u8; 12] {
    let mut bytes = [0u8; 12];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

// #[test]
// fn lmao() {
//     let secret = "abcdef";
//     encode_decode("tmp.png", "out/out.png", "out/cool.bin", secret).unwrap();
//     assert!(false)
// }

#[test]
fn round_trip() {
    let in_text = "hallo bye";
    let in_file = io::Cursor::new(in_text);
    let mut between_file = Vec::new();
    let mut out_file = Vec::new();
    let password = "secret";

    encode(in_file, &mut between_file, &Key::from(password)).unwrap();
    decode(
        io::Cursor::new(between_file),
        &mut out_file,
        &Key::from(password),
    )
    .unwrap();

    let out_text = String::from_utf8(out_file).unwrap();
    assert_eq!(in_text, out_text);
}
