use super::models::{OnlineSource, OnlineWallpaperItem};
use serde::Deserialize;
use std::time::Duration;

const API_URL: &str = "https://commons.wikimedia.org/w/api.php";
const PAGE_SIZE: u32 = 30;
const MAX_IMAGE_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    query: Option<SearchQuery>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    #[serde(default)]
    pages: Vec<CommonsPage>,
}

#[derive(Debug, Deserialize)]
struct CommonsPage {
    pageid: u64,
    title: String,
    #[serde(default)]
    imageinfo: Vec<ImageInfo>,
}

#[derive(Debug, Deserialize)]
struct ImageInfo {
    url: String,
    thumburl: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    size: Option<u64>,
    extmetadata: Option<CommonsMetadata>,
}

#[derive(Debug, Deserialize)]
struct CommonsMetadata {
    #[serde(rename = "Artist")]
    artist: Option<MetadataValue>,
    #[serde(rename = "LicenseShortName")]
    licenseshortname: Option<MetadataValue>,
}

#[derive(Debug, Deserialize)]
struct MetadataValue {
    value: Option<String>,
}

pub async fn fetch_wikimedia_wallpapers(
    client: &reqwest::Client,
    page: u32,
    query: &str,
) -> Result<Vec<OnlineWallpaperItem>, String> {
    let search = query.trim();
    let search = if search.is_empty() { "desktop background landscape" } else { search };
    let offset = page.saturating_sub(1).saturating_mul(PAGE_SIZE);
    let offset = offset.to_string();
    let response = client
        .get(API_URL)
        .query(&[
            ("action", "query"),
            ("generator", "search"),
            ("gsrsearch", search),
            ("gsrnamespace", "6"),
            ("gsrlimit", "30"),
            ("gsroffset", offset.as_str()),
            ("prop", "imageinfo"),
            ("iiprop", "url|size|extmetadata"),
            ("iiurlwidth", "720"),
            ("format", "json"),
            ("formatversion", "2"),
        ])
        .header("User-Agent", concat!("SwordWallpaper/", env!("CARGO_PKG_VERSION"), " (Wikimedia Commons wallpaper search)"))
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|error| format!("Wikimedia Commons request failed: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("Wikimedia Commons returned HTTP {}", response.status()));
    }

    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read Wikimedia Commons response: {error}"))?;
    parse_wikimedia_search(&body)
}

fn parse_wikimedia_search(body: &str) -> Result<Vec<OnlineWallpaperItem>, String> {
    let response: SearchResponse = serde_json::from_str(body)
        .map_err(|error| format!("Could not parse Wikimedia Commons response: {error}"))?;
    let pages = response.query.map(|query| query.pages).unwrap_or_default();
    let mut items = Vec::new();

    for page in pages {
        let Some(info) = page.imageinfo.into_iter().next() else {
            continue;
        };
        let extension = page
            .title
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
            .unwrap_or_default();
        if !["jpg", "jpeg", "png", "webp"].contains(&extension.as_str())
            || info.size.unwrap_or(u64::MAX) > MAX_IMAGE_BYTES
            || info.width.unwrap_or(0) < 1280
            || info.height.unwrap_or(0) < 720
        {
            continue;
        }

        let Some(thumb_url) = info.thumburl.filter(|url| !url.is_empty()) else {
            continue;
        };
        let title = page
            .title
            .strip_prefix("File:")
            .unwrap_or(&page.title)
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(&page.title);
        let metadata = info.extmetadata.unwrap_or(CommonsMetadata {
            artist: None,
            licenseshortname: None,
        });
        let artist = metadata
            .artist
            .and_then(|value| value.value)
            .map(|value| plain_text(&value))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Autor no indicado".into());
        let license = metadata
            .licenseshortname
            .and_then(|value| value.value)
            .map(|value| plain_text(&value))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Revisar licencia".into());
        let resolution = match (info.width, info.height) {
            (Some(width), Some(height)) => format!("{width} × {height}"),
            _ => "Imagen".into(),
        };

        items.push(OnlineWallpaperItem {
            id: page.pageid.to_string(),
            title: plain_text(title),
            author_or_copyright: format!("Wikimedia Commons · {artist} · {license}"),
            thumb_url,
            full_url: info.url,
            resolution,
            source: OnlineSource::Wikimedia,
            date: None,
        });
    }

    if items.is_empty() {
        return Err("Wikimedia Commons did not return large, supported images for this search.".into());
    }
    Ok(items)
}

fn plain_text(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_static_wallpaper_with_author_and_license() {
        let body = r#"{"query":{"pages":[{"pageid":7,"title":"File:Alpine wallpaper.jpg","imageinfo":[{"url":"https://upload.wikimedia.org/alpine.jpg?download=1","thumburl":"https://thumb.wikimedia.org/alpine.jpg","width":3840,"height":2160,"size":1200,"mime":"image/jpeg","descriptionurl":"https://commons.wikimedia.org/wiki/File:Alpine_wallpaper.jpg","extmetadata":{"Artist":{"value":"<a>Jane &amp; Sam</a>"},"LicenseShortName":{"value":"CC BY-SA 4.0"}}}]}]}}"#;
        let items = parse_wikimedia_search(body).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Alpine wallpaper");
        assert!(items[0].author_or_copyright.contains("Jane & Sam"));
        assert!(items[0].author_or_copyright.contains("CC BY-SA 4.0"));
        assert_eq!(items[0].extension(), "jpg");
        assert_eq!(items[0].source_prefix(), "commons");
    }

    #[test]
    fn skips_small_and_video_files() {
        let body = r#"{"query":{"pages":[{"pageid":1,"title":"File:small.jpg","imageinfo":[{"url":"https://upload.wikimedia.org/small.jpg","thumburl":"https://thumb.wikimedia.org/small.jpg","width":640,"height":480,"size":1000}]},{"pageid":2,"title":"File:clip.webm","imageinfo":[{"url":"https://upload.wikimedia.org/clip.webm","thumburl":"https://thumb.wikimedia.org/clip.jpg","width":1920,"height":1080,"size":1000,"mime":"video/webm"}]}]}}"#;
        assert!(parse_wikimedia_search(body).is_err());
    }
}
