use crate::prelude::*;
use crate::{err_ctx, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::any::type_name;

pub(crate) fn secure_encode(data: &(impl Serialize + ?Sized)) -> String {
    // TODO: to make this secure we have to use a private key when encoding
    base64::encode(to_json_string(data))

    // TODO: encode the query callback data
    // fn encode_token() -> String {
    //     use aes_gcm_siv::aead::{Aead, NewAead};
    //     use aes_gcm_siv::{Aes256GcmSiv, Key, Nonce};

    //     let seed: [u8; 32] = rand::random();
    //     let cipher = Aes256GcmSiv::new(&seed.into());

    //     let nonce = Nonce::from_slice(b"unique nonce"); // 96-bits; unique per message

    //     let ciphertext = cipher
    //         .encrypt(nonce, b"plaintext message".as_ref())
    //         .expect("encryption failure");

    //     let plaintext = cipher
    //         .decrypt(nonce, ciphertext.as_ref())
    //         .expect("decryption failure");

    //     // assert_eq!(&plaintext, b"plaintext message");
}

pub(crate) fn secure_decode<T: DeserializeOwned>(input: &str) -> Result<T> {
    let bytes = decode_base64(input)?;
    let json_string = std::str::from_utf8(&bytes).map_err(err_ctx!(DeserializeError::Utf8 {
        input: bytes.to_vec()
    }))?;

    from_json_string(json_string)
}

pub(crate) fn from_json_string<'a, T: Deserialize<'a>>(input: &'a str) -> Result<T> {
    serde_json::from_str(input).map_err(err_ctx!(DeserializeError::Json {
        input: input.to_owned(),
        target_ty: type_name::<T>()
    }))
}

pub(crate) fn to_json_string(data: &(impl Serialize + ?Sized)) -> String {
    serialize(data, serde_json::to_string)
}

pub(crate) fn to_json_string_pretty(data: &(impl Serialize + ?Sized)) -> String {
    serialize(data, serde_json::to_string_pretty)
}

pub(crate) fn to_yaml_string(data: &(impl Serialize + ?Sized)) -> String {
    serialize(data, serde_yaml::to_string)
}

fn serialize<T, E>(data: &T, imp: fn(&T) -> Result<String, E>) -> String
where
    T: Serialize + ?Sized,
    E: std::error::Error,
{
    imp(data).unwrap_or_else(|err| {
        let data_type = type_name::<T>();
        panic!(
            "Can't serialize data of type {data_type}: {}",
            err.display_chain()
        )
    })
}

fn decode_base64(input: &str) -> Result<Vec<u8>> {
    base64::decode(input).map_err(err_ctx!(DeserializeError::Base64 {
        input: input.to_owned()
    }))
}

/// Ingest a given string value with SHA2 hashing algorithm and base64-encode
/// the result. This gives up to 44 characters in length.
///
/// The calculation goes this: sha256 returns 256 bits. Base64 is 6 bits per
/// character (2**6 = 64). So, 256 / 6 = 42.6666, and this rounds up to 44 due
/// to base64 padding.
pub(crate) fn encode_base64_sha2(val: &str) -> String {
    base64::encode(<sha2::Sha256 as sha2::Digest>::digest(val))
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DeserializeError {
    #[error("Failed to parse JSON as `{target_ty}`, input surrounded by backticks:\n```\n{input:?}\n```")]
    Json {
        target_ty: &'static str,
        input: String,
        source: serde_json::Error,
    },

    #[error(
        "Failed to decode the input as base64, input surrounded by backticks:\n```\n{input:?}\n```"
    )]
    Base64 {
        input: String,
        source: base64::DecodeError,
    },

    #[error(
        "The input is not a valid UTF8 sequence, input in base64: {}",
        base64::encode(input)
    )]
    Utf8 {
        input: Vec<u8>,
        source: std::str::Utf8Error,
    },
}
