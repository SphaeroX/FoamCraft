// SPDX-License-Identifier: GPL-3.0-or-later
//! The baseline JPEG encoder of the video frames: 4:2:0 chroma subsampling (what the Motion JPEG
//! decoder of Windows expects), any size, and pictures that decode close to the input.

use foamcraft_render::jpeg::encode_rgb;

fn picture(w: u32, h: u32) -> Vec<u8> {
    let mut px = Vec::new();
    for y in 0..h {
        for x in 0..w {
            px.extend_from_slice(&[
                (x * 255 / w.max(2).saturating_sub(1).max(1)) as u8,
                (y * 255 / h.max(2).saturating_sub(1).max(1)) as u8,
                if (x / 8 + y / 8) % 2 == 0 { 200 } else { 60 },
            ]);
        }
    }
    px
}

fn sof0(jpeg: &[u8]) -> &[u8] {
    let at = jpeg
        .windows(2)
        .position(|w| w == [0xFF, 0xC0])
        .expect("SOF0");
    &jpeg[at..]
}

#[test]
fn frames_use_baseline_4_2_0_and_decode_close_to_the_input() {
    for (w, h) in [(64, 48), (37, 23), (1, 1), (17, 16)] {
        let rgb = picture(w, h);
        let jpeg = encode_rgb(&rgb, w, h, 90);
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8]);
        assert_eq!(&jpeg[jpeg.len() - 2..], &[0xFF, 0xD9]);
        let sof = sof0(&jpeg);
        // Height and width, three components: Y 2×2, Cb and Cr 1×1.
        assert_eq!(u16::from_be_bytes([sof[5], sof[6]]) as u32, h);
        assert_eq!(u16::from_be_bytes([sof[7], sof[8]]) as u32, w);
        assert_eq!(sof[9], 3);
        assert_eq!((sof[11], sof[14], sof[17]), (0x22, 0x11, 0x11));
        let img = image::load_from_memory_with_format(&jpeg, image::ImageFormat::Jpeg)
            .unwrap()
            .to_rgb8();
        assert_eq!(img.dimensions(), (w, h));
        let err: f64 = img
            .as_raw()
            .iter()
            .zip(&rgb)
            .map(|(a, b)| f64::from((i16::from(*a) - i16::from(*b)).abs()))
            .sum::<f64>()
            / rgb.len() as f64;
        // Chroma at half resolution blurs the checkerboard edges a little.
        assert!(err < 12.0, "{w}×{h}: mean error {err}");
    }
}

#[test]
fn a_flat_colour_stays_that_colour() {
    let (w, h) = (48, 32);
    let rgb: Vec<u8> = (0..w * h).flat_map(|_| [30u8, 160, 90]).collect();
    let jpeg = encode_rgb(&rgb, w, h, 85);
    let img = image::load_from_memory_with_format(&jpeg, image::ImageFormat::Jpeg)
        .unwrap()
        .to_rgb8();
    for p in img.pixels() {
        for (c, want) in p.0.iter().zip([30u8, 160, 90]) {
            assert!((i16::from(*c) - i16::from(want)).abs() <= 3, "{p:?}");
        }
    }
}
