//! Bindings for the [JPEG image snapsnot API](https://developer.axis.com/vapix/network-video/video-streaming/#jpeg-image-snapshot).
use anyhow::{bail, Context};

use crate::Client;

pub struct Jpg3 {
    client: Client,
}

pub struct RequestBuilder {
    client: Client,
    resolution: Option<String>,
    compression: Option<u8>,
}

impl RequestBuilder {
    /// The image resolution in the format `"{width}x{height}"`.
    ///
    /// Example: `"640x360"`.
    ///
    /// Valid values are product-dependent.
    // TODO: Document how to retrieve valid values.
    pub fn resolution(mut self, resolution: &str) -> Self {
        self.resolution = Some(resolution.to_string());
        self
    }

    /// Image compressions expressed as a number in `0..=100`.
    ///
    /// Example: `80`.
    ///
    /// A compression value yields lower image quality.
    pub fn compression(mut self, compression: u8) -> Self {
        self.compression = Some(compression);
        self
    }

    pub async fn send(self) -> anyhow::Result<Vec<u8>> {
        let Self {
            client,
            resolution,
            compression,
        } = self;
        let mut query = Vec::new();
        if let Some(resolution) = resolution.as_deref() {
            query.push(("resolution", resolution));
        }
        let compression = compression.map(|c| c.to_string());
        if let Some(compression) = compression.as_deref() {
            query.push(("compression", compression));
        }
        let resp = client
            .get("axis-cgi/jpg/image.cgi")?
            .query(&query)
            .send()
            .await?;

        let resp = resp.error_for_status()?;

        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map(|b| b.to_vec())
            .with_context(|| format!("Failed to get image; status: {status}"))?;

        let magic = b"\xFF\xD8\xFF";
        if !bytes.starts_with(magic) {
            bail!("Expected magic bytes {magic:?}, but got {:?}", &bytes[..3])
        }

        Ok(bytes)
    }
}

impl Jpg3 {
    /// Get a jpg encoded snapshot.
    pub fn get_image(self) -> RequestBuilder {
        RequestBuilder {
            client: self.client,
            resolution: None,
            compression: None,
        }
    }
}

impl Client {
    pub fn jpg_3(&self) -> Jpg3 {
        Jpg3 {
            client: self.clone(),
        }
    }
}
