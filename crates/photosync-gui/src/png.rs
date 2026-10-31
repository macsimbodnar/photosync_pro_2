//! A PNG of a window's picture, with no compression and no crate: it is read
//! once, by a test or an agent, and `png` is only a dev-dependency of iced_test,
//! so the shipped binary would gain it for one hidden flag (DEC-072 (2)).

use std::fs;
use std::io;
use std::path::Path;

/// The most one stored deflate block can hold.
const BLOCK: usize = 65535;

/// `rgba` as a PNG, 8 bits per channel, no filter.
pub fn encode(width: u32, height: u32, rgba: &[u8]) -> io::Result<Vec<u8>> {
    let row = width as usize * 4;
    if rgba.len() != row * height as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} bytes is not a picture of {width}x{height}", rgba.len()),
        ));
    }
    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    // An empty picture has no rows to split.
    for line in rgba.chunks(row.max(1)) {
        raw.push(0);
        raw.extend_from_slice(line);
    }

    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    // 8 bits, RGBA, deflate, no filter method but the base one, not interlaced.
    header.extend_from_slice(&[8, 6, 0, 0, 0]);

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &zlib_stored(&raw));
    chunk(&mut png, b"IEND", &[]);
    Ok(png)
}

/// `encode`, to a file.
pub fn write(path: &Path, width: u32, height: u32, rgba: &[u8]) -> io::Result<()> {
    fs::write(path, encode(width, height, rgba)?)
}

fn chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(kind);
    png.extend_from_slice(data);
    let mut checked = kind.to_vec();
    checked.extend_from_slice(data);
    png.extend_from_slice(&crc32(&checked).to_be_bytes());
}

fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let blocks = raw.len().div_ceil(BLOCK).max(1);
    let mut rest = raw;
    for at in 0..blocks {
        let (block, after) = rest.split_at(rest.len().min(BLOCK));
        rest = after;
        out.push(u8::from(at + 1 == blocks));
        out.extend_from_slice(&(block.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

pub(crate) fn crc32(bytes: &[u8]) -> u32 {
    let table: [u32; 256] = std::array::from_fn(|n| {
        (0..8).fold(n as u32, |c, _| {
            if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            }
        })
    });
    !bytes.iter().fold(!0u32, |c, b| {
        table[((c ^ u32::from(*b)) & 0xFF) as usize] ^ (c >> 8)
    })
}

pub(crate) fn adler32(bytes: &[u8]) -> u32 {
    let (a, b) = bytes.iter().fold((1u32, 0u32), |(a, b), byte| {
        let a = (a + u32::from(*byte)) % 65521;
        (a, (b + a) % 65521)
    });
    (b << 16) | a
}
