use super::HttpClientError;
use crate::prelude::*;
use crate::temp_file::create_temp_file;
use crate::{err, err_ctx, Result};
use async_trait::async_trait;
use bytes::Bytes;
use easy_ext::ext;
use futures::prelude::*;
use reqwest::Response;
use reqwest_middleware::RequestBuilder;
use tokio::io::AsyncWriteExt;

#[ext(RequestBuilderBasicExt)]
#[async_trait]
pub(crate) impl RequestBuilder {
    /// Same as [`Self::try_send()`], but requires the response to have the
    /// content length header, and returns its value along with the response.
    async fn try_send_with_content_length(self) -> Result<(Response, u64)> {
        let response = self.try_send().await?;

        let url = response.url();

        let content_length = response
            .content_length()
            .fatal_ctx(|| format!("No content length header was returned from {url}"))?;

        Ok((response, content_length))
    }

    /// Better version of [`RequestBuilder::send`] that returns an error
    /// if the error response status code is returned.
    async fn try_send(self) -> Result<Response> {
        let response = self
            .send()
            .await
            .map_err(err_ctx!(HttpClientError::Request))?;

        let status = response.status();

        if !status.is_client_error() && !status.is_server_error() {
            return Ok(response);
        }

        let body = response.text().await.unwrap_or_else(|err| {
            format!(
                "Could not collect the error response body text: {}",
                err.display_chain()
            )
        });

        Err(err!(HttpClientError::BadResponseStatusCode {
            status,
            body
        }))
    }

    async fn read_bytes(self) -> Result<Bytes> {
        self
            .try_send()
            .await?
            .bytes()
            .await
            .fatal_ctx(|| "Failed to read bytes from HTTP response")
    }
}

#[ext(ResponseBasicExt)]
#[async_trait]
pub(crate) impl Response {
    async fn read_to_temp_file(self) -> Result<tempfile::TempPath> {
        let (mut file, path) = create_temp_file().await?.into_tokio();

        self.read_to_file_handle(&mut file).await?;

        Ok(path)
    }

    async fn read_to_file_handle(self, file_handle: &mut tokio::fs::File) -> Result {
        let mut stream = self.bytes_stream();

        let mut file_handle = tokio::io::BufWriter::with_capacity(
            1024 * 1024, // 1 MB
            file_handle,
        );

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(err_ctx!(HttpClientError::ReadPayload))?;
            file_handle
                .write_all(&chunk)
                .await
                .fatal_ctx(|| format!("Failed to write HTTP stream chunk to file"))?;
        }

        file_handle
            .flush()
            .await
            .fatal_ctx(|| format!("Failed to flush file created for HTTP stream"))?;

        Ok(())
    }
}
