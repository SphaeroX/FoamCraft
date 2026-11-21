// SPDX-License-Identifier: GPL-3.0-or-later
//! Video files: Motion JPEG in an AVI container. The file is a valid RIFF/AVI with the right
//! frame count and rate, an index, and frames that decode to the pictures that went in.

use foamcraft_render::video::AviWriter;

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn find(b: &[u8], tag: &[u8]) -> usize {
    b.windows(tag.len())
        .position(|w| w == tag)
        .unwrap_or_else(|| panic!("{tag:?} missing"))
}

fn frame(w: u32, h: u32, shade: u8) -> Vec<u8> {
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            px.extend_from_slice(&[shade, (x * 255 / w) as u8, (y * 255 / h) as u8, 255]);
        }
    }
    px
}

#[test]
fn an_avi_file_has_header_index_and_decodable_frames() {
    let dir = std::env::temp_dir().join(format!("fc-video-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("clip.avi");
    let (w, h, fps, n) = (64u32, 48u32, 25u32, 10u32);
    let mut writer = AviWriter::create(&path, w, h, fps, 90).unwrap();
    for i in 0..n {
        writer.push_rgba(&frame(w, h, (i * 20) as u8)).unwrap();
    }
    writer.finish().unwrap();

    let b = std::fs::read(&path).unwrap();
    assert_eq!(&b[0..4], b"RIFF");
    assert_eq!(u32_at(&b, 4) as usize, b.len() - 8);
    assert_eq!(&b[8..12], b"AVI ");
    // Main header: microseconds per frame, total frames, size.
    let avih = find(&b, b"avih") + 8;
    assert_eq!(u32_at(&b, avih), 1_000_000 / fps);
    assert_eq!(u32_at(&b, avih + 16), n);
    assert_eq!(u32_at(&b, avih + 32), w);
    assert_eq!(u32_at(&b, avih + 36), h);
    // Stream header: video, Motion JPEG, rate/scale = fps, length = frames.
    let strh = find(&b, b"strh") + 8;
    assert_eq!(&b[strh..strh + 4], b"vids");
    assert_eq!(&b[strh + 4..strh + 8], b"MJPG");
    assert_eq!(u32_at(&b, strh + 24) / u32_at(&b, strh + 20), fps);
    assert_eq!(u32_at(&b, strh + 32), n);
    // One index entry per frame.
    let idx = find(&b, b"idx1");
    assert_eq!(u32_at(&b, idx + 4) as usize / 16, n as usize);
    // Every frame chunk is a JPEG of the right size whose colour matches what went in.
    let mut at = find(&b, b"movi") + 4;
    for i in 0..n {
        assert_eq!(&b[at..at + 4], b"00dc");
        let len = u32_at(&b, at + 4) as usize;
        let jpeg = &b[at + 8..at + 8 + len];
        let img = image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg)
            .unwrap()
            .to_rgb8();
        assert_eq!(img.dimensions(), (w, h));
        let red = img.get_pixel(w / 2, h / 2)[0];
        assert!(
            (i32::from(red) - (i * 20) as i32).abs() <= 6,
            "frame {i}: {red}"
        );
        at += 8 + len + (len & 1);
    }
}

#[test]
fn the_same_frames_give_the_same_file() {
    let dir = std::env::temp_dir().join(format!("fc-video-same-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let write = |name: &str| {
        let path = dir.join(name);
        let mut w = AviWriter::create(&path, 32, 32, 30, 85).unwrap();
        for i in 0..5 {
            w.push_rgba(&frame(32, 32, i * 40)).unwrap();
        }
        w.finish().unwrap();
        std::fs::read(path).unwrap()
    };
    assert_eq!(write("a.avi"), write("b.avi"));
}
