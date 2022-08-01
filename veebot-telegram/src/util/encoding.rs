use crate::util::prelude::*;
use crate::{err_ctx, DeserializeError, Result};
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

pub(crate) fn to_json_string<T: Serialize + ?Sized>(data: &T) -> String {
    serde_json::to_string(&data).unwrap_or_else(|err| {
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
