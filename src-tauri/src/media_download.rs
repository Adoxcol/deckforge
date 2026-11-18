//! Bounded downloads for user-selected media links. Decoding stays in the media worker.
const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonsGif {
    title: String,
    thumbnail_url: String,
    image_url: String,
    source_url: String,
    size: u64,
}

fn commons_gifs(value: &serde_json::Value) -> Vec<CommonsGif> {
    value["query"]["pages"].as_array().into_iter().flatten().filter_map(|page| {
        let info = page["imageinfo"].as_array()?.first()?;
        if info["mime"].as_str()? != "image/gif" { return None; }
        let size = info["size"].as_u64()?;
        if size > MAX_BYTES as u64 { return None; }
        let safe_url = |field: &str| -> Option<String> {
            let url = reqwest::Url::parse(info[field].as_str()?).ok()?;
            if url.scheme() != "https" || !matches!(url.host_str()?, "upload.wikimedia.org" | "thumb.wikimedia.org") { return None; }
            Some(url.to_string())
        };
        let image_url = safe_url("url")?;
        let thumbnail_url = safe_url("thumburl").unwrap_or_else(|| image_url.clone());
        let source_url = info["descriptionurl"].as_str()?.to_string();
        if !source_url.starts_with("https://commons.wikimedia.org/wiki/File:") { return None; }
        Some(CommonsGif {
            title: page["title"].as_str()?.trim_start_matches("File:").to_string(),
            thumbnail_url, image_url, source_url, size,
        })
    }).collect()
}

#[tauri::command]
pub async fn search_commons_gifs(query: String) -> Result<Vec<CommonsGif>, String> {
    let query = query.trim();
    if query.len() < 2 || query.len() > 80 || query.chars().any(char::is_control) {
        return Err("Enter 2–80 characters to search for GIFs".into());
    }
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let mut url = reqwest::Url::parse("https://commons.wikimedia.org/w/api.php").map_err(|e| e.to_string())?;
    url.query_pairs_mut().extend_pairs([
        ("action", "query"), ("format", "json"), ("formatversion", "2"),
        ("generator", "search"), ("gsrsearch", &format!("{query} filemime:\"image/gif\" filesize:<8192")),
        ("gsrnamespace", "6"), ("gsrlimit", "18"), ("prop", "imageinfo"),
        ("iiprop", "url|size|mime"), ("iiurlwidth", "240"),
    ]);
    let client = reqwest::Client::builder().https_only(true)
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("DeckForge/0.4.9 (https://github.com/Adoxcol/deckforge)")
        .build().map_err(|e| e.to_string())?;
    let response = client.get(url).send().await.map_err(|_| "Could not search Commons. Check your connection.".to_string())?
        .error_for_status().map_err(|e| format!("Commons search returned {}", e.status().map(|s| s.as_u16()).unwrap_or(0)))?;
    if response.content_length().is_some_and(|size| size > 1024 * 1024) { return Err("Commons response was too large".into()); }
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 { return Err("Commons response was too large".into()); }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| "Commons returned invalid search results".to_string())?;
    if value.get("error").is_some() { return Err("Commons could not complete that search".into()); }
    Ok(commons_gifs(&value))
}

#[tauri::command]
pub fn open_commons_file_page(url: String) -> Result<(), String> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| "Invalid Commons file page".to_string())?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("commons.wikimedia.org")
        || !parsed.path().starts_with("/wiki/File:") || !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("Invalid Commons file page".into());
    }
    crate::actions::launch_app_or_url(parsed.as_str()).map(|_| ())
}

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
    #[test]
    fn commons_results_accept_only_small_gifs_from_wikimedia_hosts() {
        let value = serde_json::json!({"query":{"pages":[
            {"title":"File:Cat.gif","imageinfo":[{"mime":"image/gif","size":1234,"url":"https://upload.wikimedia.org/cat.gif","thumburl":"https://thumb.wikimedia.org/cat.gif","descriptionurl":"https://commons.wikimedia.org/wiki/File:Cat.gif"}]},
            {"title":"File:Bad.gif","imageinfo":[{"mime":"image/gif","size":1234,"url":"https://example.com/bad.gif","descriptionurl":"https://commons.wikimedia.org/wiki/File:Bad.gif"}]},
            {"title":"File:Large.gif","imageinfo":[{"mime":"image/gif","size":9000000,"url":"https://upload.wikimedia.org/large.gif","descriptionurl":"https://commons.wikimedia.org/wiki/File:Large.gif"}]}
        ]}});
        let found = commons_gifs(&value);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Cat.gif");
    }
    #[test]
    fn commons_file_page_rejects_other_sites() {
        assert!(open_commons_file_page("https://example.com/wiki/File:Cat.gif".into()).is_err());
        assert!(open_commons_file_page("https://commons.wikimedia.org/wiki/Category:Cats".into()).is_err());
    }
    #[test]
    #[ignore = "queries Wikimedia Commons over the network"]
    fn live_commons_gif_search() {
        let found = tauri::async_runtime::block_on(search_commons_gifs("cat".into())).unwrap();
        assert!(!found.is_empty());
        assert!(found.iter().all(|gif| gif.image_url.starts_with("https://upload.wikimedia.org/")));
    }
}
