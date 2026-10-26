//! Logo d'un client (mode MSP) préparé pour le PDF du rapport SLA.
//!
//! JPEG : incorporé tel quel, seules ses dimensions sont lues. PNG 8 bits non
//! entrelacé (gris, RVB, avec ou sans alpha) : décompressé, défiltré, puis
//! recompressé en pixels bruts et masque alpha séparé. Le reste (WebP, PNG à
//! palette ou 16 bits) n'est pas incorporé : le nom du client en tient lieu.
//!
//! L'entrée vient d'un administrateur mais reste bornée comme une entrée
//! hostile : dimensions plafonnées avant toute allocation, décompression
//! limitée à la taille attendue (pas de bombe zlib).

use std::io::{Read, Write};

use super::pdf::Image;

/// Pixels au plus : 2 Mpx, bien plus qu'il n'en faut pour un logo.
const MAX_PIXELS: u64 = 2_000_000;

/// Prépare un logo pour le PDF, ou `None` s'il ne peut pas être incorporé.
pub fn for_pdf(bytes: &[u8]) -> Option<Image> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        jpeg(bytes)
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        png(bytes)
    } else {
        None
    }
}

fn jpeg(bytes: &[u8]) -> Option<Image> {
    // Parcours des segments jusqu'au premier SOF (début de trame).
    let mut at = 2;
    while at + 4 <= bytes.len() {
        if bytes[at] != 0xFF {
            return None;
        }
        let marker = bytes[at + 1];
        if marker == 0xFF {
            at += 1;
            continue;
        }
        let length = usize::from(u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]));
        // SOF0 à SOF15, sauf DHT (C4), JPG (C8) et DAC (CC).
        if (0xC0..=0xCF).contains(&marker) && ![0xC4, 0xC8, 0xCC].contains(&marker) {
            let segment = bytes.get(at + 4..at + 2 + length)?;
            if segment.len() < 6 || segment[0] != 8 {
                return None;
            }
            let height = u32::from(u16::from_be_bytes([segment[1], segment[2]]));
            let width = u32::from(u16::from_be_bytes([segment[3], segment[4]]));
            let color_space = match segment[5] {
                1 => "DeviceGray",
                3 => "DeviceRGB",
                // CMYK (Adobe inversé) et autres : pas d'incorporation.
                _ => return None,
            };
            if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
                return None;
            }
            return Some(Image {
                width,
                height,
                color_space,
                filter: "DCTDecode",
                data: bytes.to_vec(),
                alpha: None,
            });
        }
        if length < 2 {
            return None;
        }
        at += 2 + length;
    }
    None
}

fn png(bytes: &[u8]) -> Option<Image> {
    let mut at = 8;
    let mut header: Option<(u32, u32, u8, u8, u8)> = None;
    let mut compressed: Vec<u8> = Vec::new();
    while at + 8 <= bytes.len() {
        let length = u32::from_be_bytes(bytes[at..at + 4].try_into().ok()?) as usize;
        let kind = &bytes[at + 4..at + 8];
        let data = bytes.get(at + 8..at.checked_add(8)?.checked_add(length)?)?;
        match kind {
            b"IHDR" if data.len() >= 13 => {
                let width = u32::from_be_bytes(data[0..4].try_into().ok()?);
                let height = u32::from_be_bytes(data[4..8].try_into().ok()?);
                header = Some((width, height, data[8], data[9], data[12]));
            }
            b"IDAT" => compressed.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        // Données, puis CRC sur 4 octets.
        at = at.checked_add(12)?.checked_add(length)?;
    }
    let (width, height, depth, color_type, interlace) = header?;
    if depth != 8 || interlace != 0 || width == 0 || height == 0 {
        return None;
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return None;
    }
    let channels: usize = match color_type {
        0 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        // Palette : non prise en charge.
        _ => return None,
    };
    let (w, h) = (width as usize, height as usize);
    let stride = w * channels;
    let expected = (stride + 1) * h;
    let mut raw = Vec::with_capacity(expected);
    flate2::read::ZlibDecoder::new(compressed.as_slice())
        .take(expected as u64 + 1)
        .read_to_end(&mut raw)
        .ok()?;
    if raw.len() != expected {
        return None;
    }
    let pixels = unfilter(&raw, stride, h, channels)?;

    let color_channels = if channels <= 2 { 1 } else { 3 };
    let has_alpha = channels == 2 || channels == 4;
    let mut color = Vec::with_capacity(w * h * color_channels);
    let mut alpha = Vec::with_capacity(if has_alpha { w * h } else { 0 });
    for pixel in pixels.chunks_exact(channels) {
        color.extend_from_slice(&pixel[..color_channels]);
        if has_alpha {
            alpha.push(pixel[channels - 1]);
        }
    }
    Some(Image {
        width,
        height,
        color_space: if color_channels == 1 { "DeviceGray" } else { "DeviceRGB" },
        filter: "FlateDecode",
        data: deflate(&color)?,
        alpha: if has_alpha { Some(deflate(&alpha)?) } else { None },
    })
}

/// Défait les filtres PNG ligne par ligne (aucun, Sub, Up, Average, Paeth).
fn unfilter(raw: &[u8], stride: usize, rows: usize, bpp: usize) -> Option<Vec<u8>> {
    let mut out = vec![0u8; stride * rows];
    for row in 0..rows {
        let filter = raw[row * (stride + 1)];
        let line = &raw[row * (stride + 1) + 1..(row + 1) * (stride + 1)];
        let (done, rest) = out.split_at_mut(row * stride);
        let previous = if row == 0 { None } else { Some(&done[(row - 1) * stride..]) };
        let current = &mut rest[..stride];
        for i in 0..stride {
            let left = if i >= bpp { current[i - bpp] } else { 0 };
            let up = previous.map_or(0, |p| p[i]);
            let up_left = if i >= bpp { previous.map_or(0, |p| p[i - bpp]) } else { 0 };
            let predicted = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                4 => paeth(left, up, up_left),
                _ => return None,
            };
            current[i] = line[i].wrapping_add(predicted);
        }
    }
    Some(out)
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let pa = (p - i16::from(a)).abs();
    let pb = (p - i16::from(b)).abs();
    let pc = (p - i16::from(c)).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

fn deflate(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).ok()?;
    encoder.finish().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un PNG minimal, construit à la main : `pixels` est la suite des lignes
    /// déjà préfixées de leur filtre.
    fn png_file(width: u32, height: u32, color_type: u8, scanlines: &[u8]) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut chunk = |kind: &[u8], data: &[u8]| {
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            out.extend_from_slice(kind);
            out.extend_from_slice(data);
            out.extend_from_slice(&[0, 0, 0, 0]);
        };
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, color_type, 0, 0, 0]);
        chunk(b"IHDR", &ihdr);
        chunk(b"IDAT", &deflate(scanlines).unwrap());
        chunk(b"IEND", &[]);
        out
    }

    fn inflate(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        flate2::read::ZlibDecoder::new(bytes).read_to_end(&mut out).unwrap();
        out
    }

    #[test]
    fn an_rgba_png_is_split_into_colour_and_alpha() {
        // 2×2 : ligne 1 sans filtre, ligne 2 filtrée « Up ».
        let lines = [
            0, 10, 20, 30, 255, 40, 50, 60, 128, //
            2, 1, 1, 1, 0, 1, 1, 1, 0,
        ];
        let image = for_pdf(&png_file(2, 2, 6, &lines)).expect("decoded");
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(image.color_space, "DeviceRGB");
        assert_eq!(inflate(&image.data), vec![10, 20, 30, 40, 50, 60, 11, 21, 31, 41, 51, 61]);
        assert_eq!(inflate(&image.alpha.unwrap()), vec![255, 128, 255, 128]);
    }

    #[test]
    fn sub_and_paeth_filters_are_undone() {
        // 2×1 gris : Sub ; puis 2×2 gris avec Paeth sur la 2e ligne.
        let image = for_pdf(&png_file(2, 1, 0, &[1, 5, 3])).expect("decoded");
        assert_eq!(inflate(&image.data), vec![5, 8]);
        assert!(image.alpha.is_none());
        let image = for_pdf(&png_file(2, 2, 0, &[0, 5, 8, 4, 1, 1])).expect("decoded");
        // Ligne 2 : 5+1 = 6 (prédit : haut = 5), puis Paeth(6, 8, 5) = 8 → 9.
        assert_eq!(inflate(&image.data), vec![5, 8, 6, 9]);
    }

    #[test]
    fn hostile_or_unsupported_files_are_refused() {
        // Trop de pixels annoncés : refusé avant toute décompression.
        assert!(for_pdf(&png_file(5000, 5000, 6, &[0])).is_none());
        // Données tronquées.
        assert!(for_pdf(&png_file(4, 4, 2, &[0, 1, 2])).is_none());
        // Palette.
        assert!(for_pdf(&png_file(1, 1, 3, &[0, 0])).is_none());
        assert!(for_pdf(b"RIFF\0\0\0\0WEBPVP8 ").is_none());
        assert!(for_pdf(b"").is_none());
    }

    #[test]
    fn a_jpeg_keeps_its_bytes_and_reads_its_size() {
        // SOI, APP0 minimal, SOF0 (8 bits, 3×2, 3 composantes).
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
        jpeg.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 8, 0, 2, 0, 3, 3]);
        jpeg.extend_from_slice(&[1, 0x11, 0, 2, 0x11, 1, 3, 0x11, 1]);
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        let image = for_pdf(&jpeg).expect("decoded");
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(image.filter, "DCTDecode");
        assert_eq!(image.data, jpeg);
    }
}
