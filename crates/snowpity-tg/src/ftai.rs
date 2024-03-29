//! Symbols related to communicating with the 15.ai API

use crate::prelude::*;
use crate::{err, err_ctx, Result};
use crate::{http, util};

/// Limit of the text length that can be passed to 15.ai for voice generation
pub(crate) const MAX_TEXT_LENGTH: usize = 200;

/// Declarations of the 15.ai JSON API types.
/// They were reverse-engineered from the website's network requests.
///
/// 15.ai doesn't have a stable API, so this is a best-effort attempt,
/// any time the schema can break, so the code should be fault-tolerant.
pub(crate) mod rpc {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize)]
    pub(crate) struct GetAudioFile5Request<'s> {
        /// The string must not contain raw numbers (digit characters)
        pub(crate) text: &'s str,

        /// Name of the character to generate the audio for.
        pub(crate) character: &'s str,

        /// Set to `Contextual` by default
        pub(crate) emotion: &'s str,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct GetAudioFile5Response {
        pub(crate) wav_names: Vec<String>,
        // There are a bunch of other fields in the response that we don't use
    }
}

util::url::def!(ftai_api, "https://api.15.ai/app");
util::url::def!(ftai_cdn, "https://cdn.15.ai");

pub(crate) struct FtaiService {
    http_client: http::Client,
}

impl FtaiService {
    pub(crate) fn new(http_client: http::Client) -> Self {
        Self { http_client }
    }

    /// Generate the audio for the given text and character.
    /// The URL of the `.wav` file will be returned.
    /// It will take considerable time (tens of seconds) to finish the request!
    pub(crate) async fn generate_audio(&self, character: &str, text: &str) -> Result<Ogg> {
        let res: rpc::GetAudioFile5Response = self
            .http_client
            .post(ftai_api(["getAudioFile5"]))
            .send_and_read_json(rpc::GetAudioFile5Request {
                text,
                character,
                emotion: "Contextual",
            })
            .await
            .map_err(Box::new)
            .map_err(err_ctx!(FtAiError::Service))?;

        let wav_file = res
            .wav_names
            .into_iter()
            .next()
            .ok_or_else(|| err!(FtAiError::MissingWavFile))?;

        let url = ftai_cdn(["audio", &wav_file]);

        debug!(url = url.to_string().as_str(), "Using generated wav fale");

        let audio = self.http_client.get(url).read_bytes().await?;

        let mut reader = wav_io::reader::Reader::from_vec(audio.to_vec())
            .map_err(|message| err!(FtAiError::CreateWavReader { message }))?;

        let header = reader
            .read_header()
            .map_err(|message| err!(FtAiError::ReadWavHeader { message }))?;

        let data = reader
            .get_samples_f32()
            .map_err(|message| err!(FtAiError::ReadWavSamples { message }))?;

        // This seems to give the best quality. The original sample rate
        // of 15.ai is 44_100.
        const SAMPLE_RATE: u32 = 48_000;

        let data = wav_io::resample::linear(data, header.channels, header.sample_rate, SAMPLE_RATE);

        let wav_data: Vec<_> = data.map_collect(|f32| (f32 * i16::MAX as f32) as i16);

        let ogg = ogg_opus::encode::<SAMPLE_RATE, 1>(&wav_data)
            .map_err(err_ctx!(FtAiError::EncodeWavToOpus))?;

        Ok(Ogg { data: ogg.into() })
    }
}

#[derive(Debug)]
pub(crate) struct Ogg {
    pub(crate) data: bytes::Bytes,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FtAiError {
    #[error("15.ai returned zero WAV files in the response")]
    MissingWavFile,

    #[error(
        "Failed to create a WAV reader, that is probably a bug, it must be infallible: {message}"
    )]
    CreateWavReader { message: &'static str },

    #[error("Failed to read WAV header returned by 15.ai: {message}")]
    ReadWavHeader { message: &'static str },

    #[error("Failed to read WAV samples returned by 15.ai: {message}")]
    ReadWavSamples { message: &'static str },

    #[error("Failed to encode the resampled WAV to OGG")]
    EncodeWavToOpus { source: ogg_opus::Error },

    #[error("Invalid input. Please check the name of the character on 15.ai website, or check your input for typos.")]
    Service { source: Box<crate::Error> },
}
