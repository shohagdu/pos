//! Deep check of an uploaded image before it is stored.
//!
//! Reading the first bytes is not enough: a file can start like a JPEG and
//! carry an HTML page or a script after it (a "polyglot"), or hide markup in
//! a metadata block. So each format is walked from start to end:
//!
//! - JPEG: markers and segment lengths up to the end-of-image marker;
//! - PNG: every chunk with its CRC, `IHDR` first, `IDAT` present, `IEND` last;
//! - WebP: the RIFF size and every chunk inside it.
//!
//! Nothing may follow the end of the image (zero padding aside), and the bytes
//! may not contain markup or script (`<script`, `<html`, `<svg`, `<?php`,
//! `javascript:` …), in any letter case.

/// Why an upload was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageProblem {
    /// Not a JPG, PNG or WebP.
    Unsupported,
    /// Starts like an image but is broken, cut off or has data after its end.
    Damaged,
    /// Contains markup or script.
    Unsafe,
}

/// HTML tags that have no place in a photo, matched as `<tag` or `</tag`
/// followed by a space, `>` or `/` (so random image data does not match).
const UNSAFE_TAGS: &[&[u8]] =
    &[b"script", b"html", b"body", b"head", b"svg", b"iframe", b"object", b"embed", b"form", b"meta", b"link", b"style", b"img"];
/// Script openers matched anywhere (long enough not to occur by chance).
const UNSAFE_TEXT: &[&[u8]] = &[b"<?php", b"javascript:", b"vbscript:", b"onerror=", b"onload="];

/// The checked image's content type and file extension.
pub fn inspect(bytes: &[u8]) -> Result<(&'static str, &'static str), ImageProblem> {
    let (kind, end) = match bytes {
        [0xFF, 0xD8, 0xFF, ..] => (("image/jpeg", "jpg"), jpeg_end(bytes)),
        [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, ..] => (("image/png", "png"), png_end(bytes)),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => (("image/webp", "webp"), webp_end(bytes)),
        _ => return Err(ImageProblem::Unsupported),
    };
    let end = end.ok_or(ImageProblem::Damaged)?;
    // Some cameras pad with zeros; anything else after the image is smuggled in.
    if bytes[end..].iter().any(|&b| b != 0) {
        return Err(ImageProblem::Damaged);
    }
    if contains_markup(bytes) {
        return Err(ImageProblem::Unsafe);
    }
    Ok(kind)
}

fn contains_markup(bytes: &[u8]) -> bool {
    let lower = bytes.to_ascii_lowercase();
    if UNSAFE_TEXT.iter().any(|m| lower.windows(m.len()).any(|w| w == *m)) {
        return true;
    }
    lower.iter().enumerate().filter(|&(_, &c)| c == b'<').any(|(i, _)| {
        let rest = &lower[i + 1..];
        let rest = rest.strip_prefix(b"/").unwrap_or(rest);
        UNSAFE_TAGS.iter().any(|tag| {
            rest.strip_prefix(*tag)
                .and_then(|after| after.first())
                .is_some_and(|c| c.is_ascii_whitespace() || matches!(c, b'>' | b'/'))
        })
    })
}

fn u16_be(b: &[u8], at: usize) -> Option<usize> {
    Some(usize::from(u16::from_be_bytes([*b.get(at)?, *b.get(at + 1)?])))
}

fn u32_be(b: &[u8], at: usize) -> Option<usize> {
    let s = b.get(at..at + 4)?;
    usize::try_from(u32::from_be_bytes([s[0], s[1], s[2], s[3]])).ok()
}

fn u32_le(b: &[u8], at: usize) -> Option<usize> {
    let s = b.get(at..at + 4)?;
    usize::try_from(u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok()
}

/// Offset just past the JPEG end-of-image marker, or `None` when malformed.
fn jpeg_end(b: &[u8]) -> Option<usize> {
    let mut i = 2; // after SOI
    let mut frame = false;
    let mut scanned = false;
    loop {
        if *b.get(i)? != 0xFF {
            return None;
        }
        // Fill bytes before a marker.
        while *b.get(i + 1)? == 0xFF {
            i += 1;
        }
        let marker = *b.get(i + 1)?;
        i += 2;
        match marker {
            // The end: fine after image data (a segment may sit between the last scan and it).
            0xD9 => return scanned.then_some(i),
            0x01 | 0xD0..=0xD7 => continue,
            0x00 => return None,
            _ => {}
        }
        let len = u16_be(b, i)?;
        if len < 2 || i + len > b.len() {
            return None;
        }
        // Start of frame (not DHT C4, JPG C8 or DAC CC).
        if matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            frame = true;
        }
        i += len;
        if marker != 0xDA {
            continue;
        }
        // Start of scan: entropy-coded data until the next real marker.
        if !frame {
            return None;
        }
        scanned = true;
        loop {
            let next = b.get(i..)?.iter().position(|&x| x == 0xFF)?;
            i += next;
            let after = *b.get(i + 1)?;
            match after {
                0x00 | 0xD0..=0xD7 | 0xFF => i += if after == 0xFF { 1 } else { 2 },
                0xD9 => return Some(i + 2),
                _ => break, // another segment (progressive JPEGs have several scans)
            }
        }
    }
}

/// CRC-32 (ISO-HDLC), as PNG uses it.
fn crc32(parts: &[&[u8]]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for part in parts {
        for &byte in *part {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
            }
        }
    }
    !crc
}

/// Offset just past the PNG `IEND` chunk, or `None` when malformed.
fn png_end(b: &[u8]) -> Option<usize> {
    let mut i = 8;
    let mut first = true;
    let mut data = false;
    loop {
        let len = u32_be(b, i)?;
        let kind = b.get(i + 4..i + 8)?;
        let body = b.get(i + 8..i.checked_add(8)?.checked_add(len)?)?;
        let crc = u32_be(b, i + 8 + len)?;
        if !kind.iter().all(u8::is_ascii_alphabetic) || crc32(&[kind, body]) as usize != crc {
            return None;
        }
        if first && (kind != b"IHDR" || len != 13) {
            return None;
        }
        first = false;
        data |= kind == b"IDAT";
        i += 12 + len;
        if kind == b"IEND" {
            return data.then_some(i);
        }
    }
}

/// Offset at the end of the RIFF container, or `None` when malformed.
fn webp_end(b: &[u8]) -> Option<usize> {
    let end = u32_le(b, 4)?.checked_add(8)?;
    if end > b.len() || end < 20 {
        return None;
    }
    let mut i = 12;
    let mut first = true;
    while i < end {
        let kind = b.get(i..i + 4)?;
        let len = u32_le(b, i + 4)?;
        if first && !matches!(kind, b"VP8 " | b"VP8L" | b"VP8X") {
            return None;
        }
        first = false;
        i = i.checked_add(8)?.checked_add(len)?.checked_add(len % 2)?;
    }
    (i == end && !first).then_some(end)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A 1×1 PNG with correct CRCs.
    pub(crate) fn tiny_png() -> Vec<u8> {
        let chunk = |kind: &[u8], body: &[u8]| {
            let mut out = u32::try_from(body.len()).expect("len").to_be_bytes().to_vec();
            out.extend_from_slice(kind);
            out.extend_from_slice(body);
            out.extend_from_slice(&crc32(&[kind, body]).to_be_bytes());
            out
        };
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0]));
        png.extend(chunk(b"IDAT", &[0x78, 0x9C, 0x63, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01]));
        png.extend(chunk(b"IEND", &[]));
        png
    }

    /// A minimal baseline JPEG structure: SOI, APP0, SOF0, SOS, data, EOI.
    pub(crate) fn tiny_jpeg() -> Vec<u8> {
        let mut j = vec![0xFF, 0xD8];
        j.extend([0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0]);
        j.extend([0xFF, 0xC0, 0x00, 0x0B, 8, 0, 1, 0, 1, 1, 1, 0x11, 0]);
        j.extend([0xFF, 0xDA, 0x00, 0x08, 1, 1, 0, 0, 0x3F, 0]);
        j.extend([0x12, 0xFF, 0x00, 0x34, 0xFF, 0xD0, 0x56]);
        j.extend([0xFF, 0xD9]);
        j
    }

    fn tiny_webp() -> Vec<u8> {
        let body = [0x2F, 0, 0, 0, 0];
        let mut w = b"RIFF".to_vec();
        w.extend(u32::try_from(4 + 8 + body.len() + 1).expect("len").to_le_bytes());
        w.extend(b"WEBPVP8L");
        w.extend(u32::try_from(body.len()).expect("len").to_le_bytes());
        w.extend(body);
        w.push(0); // pad to even
        w
    }

    #[test]
    fn real_images_pass() {
        assert_eq!(inspect(&tiny_png()), Ok(("image/png", "png")));
        assert_eq!(inspect(&tiny_jpeg()), Ok(("image/jpeg", "jpg")));
        assert_eq!(inspect(&tiny_webp()), Ok(("image/webp", "webp")));
        // Some encoders write a comment between the last scan and the end.
        let mut commented = tiny_jpeg();
        commented.truncate(commented.len() - 2);
        commented.extend([0xFF, 0xFE, 0x00, 0x03, b'x', 0xFF, 0xD9]);
        assert_eq!(inspect(&commented), Ok(("image/jpeg", "jpg")));
        let mut padded = tiny_jpeg();
        padded.extend([0, 0, 0]);
        assert_eq!(inspect(&padded), Ok(("image/jpeg", "jpg")));
    }

    #[test]
    fn other_types_are_unsupported() {
        for bytes in [&b"GIF89a...."[..], b"<svg onload=alert(1)>", b"<html><script>", b"%PDF-1.7", b""] {
            assert_eq!(inspect(bytes), Err(ImageProblem::Unsupported), "{bytes:?}");
        }
    }

    #[test]
    fn a_magic_number_alone_is_not_an_image() {
        assert_eq!(inspect(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR-pretend"), Err(ImageProblem::Damaged));
        assert_eq!(inspect(&[0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3]), Err(ImageProblem::Damaged));
        assert_eq!(inspect(b"RIFF\x10\0\0\0WEBPVP8 "), Err(ImageProblem::Damaged));
        // Cut off before the end.
        let png = tiny_png();
        assert_eq!(inspect(&png[..png.len() - 4]), Err(ImageProblem::Damaged));
        let jpg = tiny_jpeg();
        assert_eq!(inspect(&jpg[..jpg.len() - 2]), Err(ImageProblem::Damaged));
        // A changed byte breaks the PNG checksum.
        let mut bad = tiny_png();
        bad[20] ^= 1;
        assert_eq!(inspect(&bad), Err(ImageProblem::Damaged));
    }

    #[test]
    fn data_after_the_image_is_refused() {
        for mut image in [tiny_png(), tiny_jpeg(), tiny_webp()] {
            image.extend(b"<html><script>alert(document.cookie)</script></html>");
            assert_eq!(inspect(&image), Err(ImageProblem::Damaged));
        }
    }

    #[test]
    fn markup_is_found_in_any_case_but_not_in_look_alikes() {
        assert!(contains_markup(b"..<ScRiPt>.."));
        assert!(contains_markup(b"..</SCRIPT>.."));
        assert!(contains_markup(b"..<svg/onload=x>.."));
        assert!(contains_markup(b"..<img src=x>.."));
        assert!(contains_markup(b"..JavaScript:alert(1).."));
        assert!(!contains_markup(b"..<scripts..<svgx..<%..<?=..<imgs.."));
    }

    #[test]
    fn markup_hidden_inside_the_image_is_refused() {
        // A JPEG comment segment carrying a script.
        let payload = b"<ScRiPt>alert(1)</script>";
        let mut jpg = vec![0xFF, 0xD8, 0xFF, 0xFE];
        jpg.extend(u16::try_from(payload.len() + 2).expect("len").to_be_bytes());
        jpg.extend(payload);
        jpg.extend(&tiny_jpeg()[2..]);
        assert_eq!(inspect(&jpg), Err(ImageProblem::Unsafe));

        // A PNG text chunk with PHP in it.
        let png = tiny_png();
        let text = b"Comment\0<?php system($_GET['c']); ?>";
        let mut chunk = u32::try_from(text.len()).expect("len").to_be_bytes().to_vec();
        chunk.extend(b"tEXt");
        chunk.extend(text);
        chunk.extend(crc32(&[b"tEXt", text]).to_be_bytes());
        let mut with_text = png[..33].to_vec();
        with_text.extend(chunk);
        with_text.extend(&png[33..]);
        assert_eq!(inspect(&with_text), Err(ImageProblem::Unsafe));
    }
}
