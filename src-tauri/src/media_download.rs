//! Bounded downloads for user-selected media links. Decoding stays in the media worker.
const MAX_BYTES: usize = 8 * 1024 * 1024;

fn parse_url(raw: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(raw.trim()).map_err(|_| "Enter a valid HTTPS image or GIF link.".to_string())?;
    if url.scheme() != "https" || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("Use an HTTPS link without embedded credentials.".into());
    }
    Ok(url)
}

fn append_chunk(bytes: &mut Vec<u8>, chunk: &[u8]) -> Result<(), String> {
    if chunk.len() > MAX_BYTES.saturating_sub(bytes.len()) {
        return Err("Image must be 8 MB or smaller.".into());
    }
    bytes.extend_from_slice(chunk);
    Ok(())
}

pub async fn download(raw: &str) -> Result<(String, Vec<u8>), String> {
    let url = parse_url(raw)?;
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("DeckForge-media-library")
        .build().map_err(|e| e.to_string())?;
    let mut response = client.get(url).send().await
        .map_err(|_| "Could not download that image. Check the link and your connection.".to_string())?
        .error_for_status().map_err(|e| format!("The image server refused the download: {}", e.status().map(|s| s.as_u16()).unwrap_or(0)))?;
    if response.content_length().is_some_and(|len| len > MAX_BYTES as u64) {
        return Err("Image must be 8 MB or smaller.".into());
    }
    let name = response.url().path_segments().and_then(|mut parts| parts.next_back())
        .filter(|name| !name.is_empty()).unwrap_or("Downloaded image").chars().take(120).collect();
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "The image download was interrupted. Try again.".to_string())? {
        append_chunk(&mut bytes, &chunk)?;
    }
    // The media decoder validates actual bytes, rather than trusting extensions or MIME headers.
    Ok((name, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_require_https_and_no_credentials() {
        assert!(parse_url(" https://example.com/image.gif?download=1 ").is_ok());
        for url in ["http://example.com/a.gif", "file:///tmp/a.gif", "data:image/gif;base64,AA", "https://user:pass@example.com/a.gif", "bad"] {
            assert!(parse_url(url).is_err(), "{url}");
        }
    }
    #[test]
    fn chunked_downloads_cannot_exceed_limit_even_without_content_length() {
        let mut bytes = vec![0; MAX_BYTES - 1];
        append_chunk(&mut bytes, &[1]).unwrap();
        assert!(append_chunk(&mut bytes, &[2]).is_err());
        assert_eq!(bytes.len(), MAX_BYTES);
        assert_eq!(bytes[MAX_BYTES - 1], 1);
    }
}
