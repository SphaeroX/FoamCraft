// SPDX-License-Identifier: GPL-3.0-or-later
//! Baseline JPEG encoder (ITU-T T.81) for video frames: YCbCr with 4:2:0 chroma subsampling,
//! the example quantisation and Huffman tables of Annex K, no restart markers. The Motion JPEG
//! decoder of Windows needs subsampled chroma; 4:4:4 frames come out with wrong colours there.

/// Zig-zag order of the 64 coefficients of a block.
const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// Annex K.1 quantisation tables (row order).
const LUMA_Q: [u16; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113,
    92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
];
const CHROMA_Q: [u16; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
];

/// Annex K.3 Huffman tables: code counts per length 1..16 and the symbols.
const DC_LUMA_BITS: [u8; 16] = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
const DC_CHROMA_BITS: [u8; 16] = [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0];
const DC_VALUES: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
const AC_LUMA_BITS: [u8; 16] = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d];
const AC_LUMA_VALUES: [u8; 162] = [
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
    0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0,
    0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
    0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
    0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5,
    0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2,
    0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
    0xf9, 0xfa,
];
const AC_CHROMA_BITS: [u8; 16] = [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77];
const AC_CHROMA_VALUES: [u8; 162] = [
    0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
    0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0,
    0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26,
    0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
    0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
    0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
    0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5,
    0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3,
    0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda,
    0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
    0xf9, 0xfa,
];

/// Code and length per symbol, from the counts per length (Annex C).
fn codes(bits: &[u8; 16], values: &[u8]) -> [(u16, u8); 256] {
    let mut table = [(0u16, 0u8); 256];
    let mut code = 0u16;
    let mut k = 0;
    for (len, count) in bits.iter().enumerate() {
        for _ in 0..*count {
            table[values[k] as usize] = (code, len as u8 + 1);
            code += 1;
            k += 1;
        }
        code <<= 1;
    }
    table
}

struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    fn put(&mut self, code: u32, len: u32) {
        self.acc = (self.acc << len) | (code & ((1 << len) - 1));
        self.n += len;
        while self.n >= 8 {
            let byte = (self.acc >> (self.n - 8)) as u8;
            self.out.push(byte);
            if byte == 0xFF {
                self.out.push(0);
            }
            self.n -= 8;
        }
        self.acc &= (1 << self.n) - 1;
    }

    fn flush(&mut self) {
        if self.n > 0 {
            let pad = 8 - self.n;
            self.put((1 << pad) - 1, pad);
        }
    }
}

/// Category and the bits of a coefficient.
fn magnitude(v: i32) -> (u32, u32) {
    let a = v.unsigned_abs();
    let size = 32 - a.leading_zeros();
    let bits = if v < 0 { (v - 1) as u32 } else { v as u32 };
    (size, bits & ((1 << size) - 1))
}

fn scaled(table: &[u16; 64], quality: u8) -> [u16; 64] {
    let q = u32::from(quality.clamp(1, 100));
    let scale = if q < 50 { 5000 / q } else { 200 - 2 * q };
    table.map(|v| ((u32::from(v) * scale + 50) / 100).clamp(1, 255) as u16)
}

/// cos((2x + 1) u π / 16) · c(u) / 2 for x, u in 0..8 (c(0) = 1/√2, else 1).
fn basis() -> &'static [[f32; 8]; 8] {
    static BASIS: std::sync::OnceLock<[[f32; 8]; 8]> = std::sync::OnceLock::new();
    BASIS.get_or_init(|| {
        let mut t = [[0f32; 8]; 8];
        for (x, row) in t.iter_mut().enumerate() {
            for (u, v) in row.iter_mut().enumerate() {
                let c = if u == 0 {
                    std::f32::consts::FRAC_1_SQRT_2
                } else {
                    1.0
                };
                *v = ((2 * x + 1) as f32 * u as f32 * std::f32::consts::PI / 16.0).cos() * c / 2.0;
            }
        }
        t
    })
}

/// Forward DCT of one 8 × 8 block (values centred on zero), separable.
fn fdct(block: &[f32; 64]) -> [f32; 64] {
    let b = basis();
    let mut tmp = [0f32; 64];
    for y in 0..8 {
        for u in 0..8 {
            tmp[y * 8 + u] = (0..8).map(|x| block[y * 8 + x] * b[x][u]).sum::<f32>();
        }
    }
    let mut out = [0f32; 64];
    for u in 0..8 {
        for v in 0..8 {
            out[v * 8 + u] = (0..8).map(|y| tmp[y * 8 + u] * b[y][v]).sum::<f32>();
        }
    }
    out
}

struct Coder {
    dc: [(u16, u8); 256],
    ac: [(u16, u8); 256],
    q: [u16; 64],
    pred: i32,
}

impl Coder {
    fn block(&mut self, bits: &mut Bits, block: &[f32; 64]) {
        let coef = fdct(block);
        let mut z = [0i32; 64];
        for (k, &i) in ZIGZAG.iter().enumerate() {
            z[k] = (coef[i] / f32::from(self.q[i])).round() as i32;
        }
        let diff = z[0] - self.pred;
        self.pred = z[0];
        let (size, value) = magnitude(diff);
        let (code, len) = self.dc[size as usize];
        bits.put(u32::from(code), u32::from(len));
        bits.put(value, size);
        let mut run = 0;
        for &c in &z[1..] {
            if c == 0 {
                run += 1;
                continue;
            }
            while run > 15 {
                let (code, len) = self.ac[0xF0];
                bits.put(u32::from(code), u32::from(len));
                run -= 16;
            }
            let (size, value) = magnitude(c);
            let (code, len) = self.ac[((run << 4) | size) as usize];
            bits.put(u32::from(code), u32::from(len));
            bits.put(value, size);
            run = 0;
        }
        if run > 0 {
            let (code, len) = self.ac[0x00];
            bits.put(u32::from(code), u32::from(len));
        }
    }
}

fn segment(out: &mut Vec<u8>, marker: u8, data: &[u8]) {
    out.extend_from_slice(&[0xFF, marker]);
    out.extend_from_slice(&((data.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(data);
}

/// Encodes tightly packed RGB8 pixels as a baseline JPEG with 4:2:0 subsampling.
pub fn encode_rgb(rgb: &[u8], width: u32, height: u32, quality: u8) -> Vec<u8> {
    let (w, h) = (width.max(1) as usize, height.max(1) as usize);
    // Full-range YCbCr (JFIF), edges repeated to whole 16 × 16 macroblocks.
    let (mw, mh) = (w.div_ceil(16) * 16, h.div_ceil(16) * 16);
    let mut ycc = vec![[0f32; 3]; mw * mh];
    for y in 0..mh {
        for x in 0..mw {
            let i = (y.min(h - 1) * w + x.min(w - 1)) * 3;
            let (r, g, b) = (
                f32::from(rgb[i]),
                f32::from(rgb[i + 1]),
                f32::from(rgb[i + 2]),
            );
            ycc[y * mw + x] = [
                0.299 * r + 0.587 * g + 0.114 * b,
                -0.168_736 * r - 0.331_264 * g + 0.5 * b + 128.0,
                0.5 * r - 0.418_688 * g - 0.081_312 * b + 128.0,
            ];
        }
    }
    let (lq, cq) = (scaled(&LUMA_Q, quality), scaled(&CHROMA_Q, quality));
    let mut out = vec![0xFF, 0xD8];
    segment(
        &mut out,
        0xE0,
        &[b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0],
    );
    for (id, table) in [(0u8, &lq), (1u8, &cq)] {
        let mut data = vec![id];
        data.extend(ZIGZAG.iter().map(|&i| table[i] as u8));
        segment(&mut out, 0xDB, &data);
    }
    let mut sof = vec![8];
    sof.extend_from_slice(&(h as u16).to_be_bytes());
    sof.extend_from_slice(&(w as u16).to_be_bytes());
    sof.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    segment(&mut out, 0xC0, &sof);
    for (class_id, bits, values) in [
        (0x00u8, &DC_LUMA_BITS, &DC_VALUES[..]),
        (0x10, &AC_LUMA_BITS, &AC_LUMA_VALUES[..]),
        (0x01, &DC_CHROMA_BITS, &DC_VALUES[..]),
        (0x11, &AC_CHROMA_BITS, &AC_CHROMA_VALUES[..]),
    ] {
        let mut data = vec![class_id];
        data.extend_from_slice(bits);
        data.extend_from_slice(values);
        segment(&mut out, 0xC4, &data);
    }
    segment(&mut out, 0xDA, &[3, 1, 0x00, 2, 0x11, 3, 0x11, 0, 63, 0]);

    let mut y_coder = Coder {
        dc: codes(&DC_LUMA_BITS, &DC_VALUES),
        ac: codes(&AC_LUMA_BITS, &AC_LUMA_VALUES),
        q: lq,
        pred: 0,
    };
    let chroma = |q| Coder {
        dc: codes(&DC_CHROMA_BITS, &DC_VALUES),
        ac: codes(&AC_CHROMA_BITS, &AC_CHROMA_VALUES),
        q,
        pred: 0,
    };
    let (mut cb_coder, mut cr_coder) = (chroma(cq), chroma(cq));
    let mut bits = Bits {
        out: Vec::with_capacity(w * h / 4),
        acc: 0,
        n: 0,
    };
    let mut block = [0f32; 64];
    for my in (0..mh).step_by(16) {
        for mx in (0..mw).step_by(16) {
            for (bx, by) in [(0, 0), (8, 0), (0, 8), (8, 8)] {
                for y in 0..8 {
                    for x in 0..8 {
                        block[y * 8 + x] = ycc[(my + by + y) * mw + mx + bx + x][0] - 128.0;
                    }
                }
                y_coder.block(&mut bits, &block);
            }
            for (c, coder) in [(1, &mut cb_coder), (2, &mut cr_coder)] {
                for y in 0..8 {
                    for x in 0..8 {
                        let p =
                            |dx: usize, dy: usize| ycc[(my + 2 * y + dy) * mw + mx + 2 * x + dx][c];
                        block[y * 8 + x] = (p(0, 0) + p(1, 0) + p(0, 1) + p(1, 1)) / 4.0 - 128.0;
                    }
                }
                coder.block(&mut bits, &block);
            }
        }
    }
    bits.flush();
    out.extend_from_slice(&bits.out);
    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}
