use std::path::Path;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;

#[derive(serde::Serialize)]
pub struct ImageData {
    pub path: String,
    pub mime_type: String,
    pub base64_data: String,
    pub width: u32,
    pub height: u32,
}

pub async fn view_image(path: &Path) -> anyhow::Result<ImageData> {
    if !path.is_absolute() {
        anyhow::bail!("path must be an absolute path");
    }

    let bytes = tokio::fs::read(path)
        .await
        .map_err(|err| anyhow::anyhow!("failed to read image file: {err}"))?;

    let img = image::load_from_memory(&bytes)
        .map_err(|err| anyhow::anyhow!("failed to decode image: {err}"))?;

    let mime_type = mime_guess::from_path(path)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let base64_data = BASE64_STANDARD.encode(&bytes);

    Ok(ImageData {
        path: path.display().to_string(),
        mime_type,
        base64_data,
        width: img.width(),
        height: img.height(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn reads_valid_image() -> anyhow::Result<()> {
        let mut temp = NamedTempFile::new()?;
        // 1x1 red PNG image
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
            0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        temp.write_all(&png_data)?;
        temp.flush()?;

        let result = view_image(temp.path()).await?;

        assert_eq!(result.width, 1);
        assert_eq!(result.height, 1);
        assert!(result.mime_type.contains("png") || result.mime_type.contains("octet-stream"));
        assert!(!result.base64_data.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn fails_on_non_image() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"not an image").unwrap();
        temp.flush().unwrap();

        let result = view_image(temp.path()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("failed to decode image"));
    }

    #[tokio::test]
    async fn fails_on_nonexistent_file() {
        let result = view_image(Path::new("/nonexistent/image.png")).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("failed to read image file"));
    }
}
