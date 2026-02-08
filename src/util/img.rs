use image::{ImageFormat, ImageReader};
use std::io::Cursor;

pub fn gambar2array(src: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let img = ImageReader::open(src)?.decode()?;

    let mut buf: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)?;

    Ok(buf)
}
