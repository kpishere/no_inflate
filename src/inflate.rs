use alloc::vec::Vec;
use core::result::Result;

use crate::inflate::bitreader::BitReader;
use crate::inflate::huffman::{HuffmanTable, build_fixed_litlen_table, build_fixed_dist_table};

pub mod bitreader;
pub mod huffman;

#[derive(Debug)]
pub enum InflateError {
    InputTooShort,
    InvalidHeader,
    Unsupported,
    BadBlockData,
    BadHuffmanCode,
    OutputOverflow,
}

pub fn inflate_zlib(input: &[u8]) -> Result<Vec<u8>, InflateError> {
    // Parse zlib header: 2 bytes
    if input.len() < 2 {
        return Err(InflateError::InputTooShort);
    }
    let cmf = input[0];
    let flg = input[1];
    // Check checkbits
    if ((cmf as u16) << 8 | flg as u16) % 31 != 0 {
        return Err(InflateError::InvalidHeader);
    }
    let cm = cmf & 0x0f;
    if cm != 8 { // DEFLATE
        return Err(InflateError::Unsupported);
    }
    let fdict = (flg & 0x20) != 0;
    if fdict {
        // We won't support preset dictionaries
        return Err(InflateError::Unsupported);
    }
    // The remaining of the buffer is deflate stream plus optional Adler32 at end.
    // We'll ignore the Adler32 check for now (future improvement).

    // Prepare bitreader to point at deflate stream starting at input[2]
    let mut br = BitReader::new(&input[2..]);
    let mut out = Vec::new();

    // Main loop over blocks
    loop {
        let bfinal = br.read_bits(1).ok_or(InflateError::InputTooShort)? as u8;
        let btype = br.read_bits(2).ok_or(InflateError::InputTooShort)? as u8;
        match btype {
            0 => { // stored block
                // Align to next byte
                br.align_to_byte();
                // read LEN and NLEN
                let lo = br.read_byte().ok_or(InflateError::InputTooShort)? as u16;
                let hi = br.read_byte().ok_or(InflateError::InputTooShort)? as u16;
                let len = (hi << 8) | lo;
                let nlo = br.read_byte().ok_or(InflateError::InputTooShort)? as u16;
                let nhi = br.read_byte().ok_or(InflateError::InputTooShort)? as u16;
                let nlen = (nhi << 8) | nlo;
                if len != (!nlen) {
                    // stored block length mismatch
                    return Err(InflateError::BadBlockData);
                }
                for _ in 0..len {
                    let b = br.read_byte().ok_or(InflateError::InputTooShort)?;
                    out.push(b);
                }
            }
            1 | 2 => {
                // Huffman compressed block (1=Fixed, 2=Dynamic)
                let litlen_table: HuffmanTable;
                let dist_table: HuffmanTable;
                if btype == 1 {
                    litlen_table = build_fixed_litlen_table();
                    dist_table = build_fixed_dist_table();
                } else {
                    // dynamic Huffman codes
                    // read HLIT, HDIST, HCLEN
                    let hlit = br.read_bits(5).ok_or(InflateError::InputTooShort)? as usize + 257;
                    let hdist = br.read_bits(5).ok_or(InflateError::InputTooShort)? as usize + 1;
                    let hclen = br.read_bits(4).ok_or(InflateError::InputTooShort)? as usize + 4;
                    let code_len_order = [16usize,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15];
                    let mut clens = [0u8; 19];
                    for i in 0..hclen {
                        clens[code_len_order[i]] = br.read_bits(3).ok_or(InflateError::InputTooShort)? as u8;
                    }
                    let cl_table = HuffmanTable::from_lengths(&clens)?;
                    // read HLIT + HDIST code lengths using the code length Huffman
                    let mut litlen_lens: Vec<u8> = Vec::new();
                    litlen_lens.resize(hlit, 0u8);
                    let mut dist_lens: Vec<u8> = Vec::new();
                    dist_lens.resize(hdist, 0u8);
                    let mut idx = 0usize;
                    while idx < hlit + hdist {
                        let sym = cl_table.read_symbol(&mut br)?;
                        match sym {
                            0..=15 => {
                                if idx < hlit { litlen_lens[idx] = sym as u8; } else { dist_lens[idx - hlit] = sym as u8; }
                                idx += 1;
                            }
                            16 => {
                                // repeat previous 3-6 times
                                let repeat = br.read_bits(2).ok_or(InflateError::InputTooShort)? + 3;
                                if idx == 0 { return Err(InflateError::BadHuffmanCode); }
                                let val = if idx <= hlit { litlen_lens[idx-1] } else { dist_lens[idx - hlit - 1] };
                                for _ in 0..repeat {
                                    if idx < hlit { litlen_lens[idx] = val; } else { dist_lens[idx - hlit] = val; }
                                    idx += 1;
                                }
                            }
                            17 => {
                                // repeat zero 3-10 times
                                let repeat = br.read_bits(3).ok_or(InflateError::InputTooShort)? + 3;
                                for _ in 0..repeat {
                                    if idx < hlit { litlen_lens[idx] = 0; } else { dist_lens[idx - hlit] = 0; }
                                    idx += 1;
                                }
                            }
                            18 => {
                                // repeat zero 11-138 times
                                let repeat = br.read_bits(7).ok_or(InflateError::InputTooShort)? + 11;
                                for _ in 0..repeat {
                                    if idx < hlit { litlen_lens[idx] = 0; } else { dist_lens[idx - hlit] = 0; }
                                    idx += 1;
                                }
                            }
                            _ => return Err(InflateError::BadHuffmanCode)
                        }
                    }
                    litlen_table = HuffmanTable::from_lengths(&litlen_lens)?;
                    dist_table = HuffmanTable::from_lengths(&dist_lens)?;
                }

                // now decode symbols until end of block
                loop {
                    let sym = litlen_table.read_symbol(&mut br)?;
                    if sym < 256 {
                        out.push(sym as u8);
                    } else if sym == 256 {
                        break;
                    } else if sym > 256 && sym <= 285 {
                        // length code
                        let (base_len, extra_bits) = match sym {
                            257 => (3, 0), 258 => (4,0), 259 => (5,0), 260 => (6,0),
                            261 => (7,0), 262 => (8,0), 263 => (9,0), 264 => (10,0),
                            265 => (11,1), 266 => (13,1), 267 => (15,1), 268 => (17,1),
                            269 => (19,2), 270 => (23,2), 271 => (27,2), 272 => (31,2),
                            273 => (35,3), 274 => (43,3), 275 => (51,3), 276 => (59,3),
                            277 => (67,4), 278 => (83,4), 279 => (99,4), 280 => (115,4),
                            281 => (131,5), 282 => (163,5), 283 => (195,5), 284 => (227,5),
                            285 => (258,0), _ => return Err(InflateError::BadHuffmanCode),
                        };
                        let extra = if extra_bits > 0 { br.read_bits(extra_bits as usize).ok_or(InflateError::InputTooShort)? as usize } else { 0 };
                        let length = base_len + extra;
                        // distance code
                        let dist_sym = dist_table.read_symbol(&mut br)?;
                        if dist_sym > 29 { return Err(InflateError::BadHuffmanCode); }
                        let (base_dist, dist_extra) = match dist_sym {
                            0 => (1,0), 1 => (2,0), 2 => (3,0), 3 => (4,0),
                            4 => (5,1), 5 => (7,1), 6 => (9,2), 7 => (13,2),
                            8 => (17,3), 9 => (25,3), 10 => (33,4), 11 => (49,4),
                            12 => (65,5), 13 => (97,5), 14 => (129,6), 15 => (193,6),
                            16 => (257,7), 17 => (385,7), 18 => (513,8), 19 => (769,8),
                            20 => (1025,9), 21 => (1537,9), 22 => (2049,10), 23 => (3073,10),
                            24 => (4097,11), 25 => (6145,11), 26 => (8193,12), 27 => (12289,12),
                            28 => (16385,13), 29 => (24577,13), _ => return Err(InflateError::BadHuffmanCode),
                        };
                        let dist_extra_val = if dist_extra > 0 { br.read_bits(dist_extra as usize).ok_or(InflateError::InputTooShort)? as usize } else { 0 };
                        let distance = base_dist + dist_extra_val;
                        // Now copy `length` bytes from distance back in `out`.
                        let current_len = out.len();
                        if distance == 0 || distance > current_len { return Err(InflateError::BadBlockData); }
                        for i in 0..length {
                            let pos = current_len - distance + i;
                            let b = out[pos];
                            out.push(b);
                        }
                    } else {
                        return Err(InflateError::BadHuffmanCode);
                    }
                }
            }
            _ => return Err(InflateError::Unsupported),
        }
        if bfinal != 0 {
            break;
        }
    }

    Ok(out)
}
