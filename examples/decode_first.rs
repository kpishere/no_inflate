use no_inflate::inflate::{bitreader::BitReader, huffman::{build_fixed_litlen_table, build_fixed_dist_table}};
use flate2::{Compression, write::ZlibEncoder};
use std::io::Write;

fn main() {
    let input = b"The quick brown fox jumps over the lazy dog";
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(input).expect("write input");
    let compressed = enc.finish().expect("finish");
    println!("compressed hex: {}", compressed.iter().map(|b| format!("{:02x}", b)).collect::<String>());

    let mut br = BitReader::new(&compressed[2..]);
    let lit_table = build_fixed_litlen_table();
    let mut dist_table = build_fixed_dist_table();

    // read a few symbols
    // Print the lookup table entries for debugging
    println!("lit table max_bits: {}", lit_table.max_bits());

    // Compute and print counts and next_code according to RFC algorithm for fixed table
    let mut counts = [0usize; 16];
    // Build lengths array for fixed table
    let mut lengths: Vec<u8> = Vec::new();
    lengths.resize(288, 0u8);
    for i in 0..=143 { lengths[i] = 8; }
    for i in 144..=255 { lengths[i] = 9; }
    for i in 256..=279 { lengths[i] = 7; }
    for i in 280..=287 { lengths[i] = 8; }
    for &l in lengths.iter() { counts[l as usize] += 1; }
    println!("counts[bits 1..=9]:");
    for bits in 1..=9 { println!("counts[{}] = {}", bits, counts[bits]); }
    let mut next_code = [0u32; 16];
    let mut code = 0u32;
    for bits in 1..=9 {
        code = (code + counts[bits - 1] as u32) << 1;
        next_code[bits] = code;
        println!("next_code[{}] = {}", bits, next_code[bits]);
    }

    // compute canonical codes for each symbol like HuffmanTable::from_lengths
    let max_bits = 9usize;
    let mut codes_arr: Vec<u32> = Vec::new();
    codes_arr.resize(max_bits + 1, 0u32);
    let mut code = 0u32;
    for bits in 1..=max_bits { code = (code + counts[bits - 1] as u32) << 1; codes_arr[bits] = code; }
    // for a symbol, compute code
    let mut sym_to_index: Vec<Option<usize>> = Vec::new();
    sym_to_index.resize(288, None);
    for (sym, &len) in lengths.iter().enumerate() {
        if len == 0 { continue; }
        let len_usz = len as usize;
        let c = codes_arr[len_usz];
        codes_arr[len_usz] += 1;
        let rev = {
            let mut v = c;
            let mut r = 0u32;
            for _ in 0..len_usz { r = (r << 1) | (v & 1); v >>= 1; }
            r
        };
        let idx = (rev as usize) << (max_bits - len_usz);
        sym_to_index[sym] = Some(idx);
        if sym == 80 { println!("sym80 c {} rev {} idx {}", c, rev, idx);}    
    }
    if let Some(idx) = sym_to_index[84] { println!("symbol 84 assigned at idx {} expecting ascii T", idx);} else { println!("symbol 84 not assigned length"); }

    // Check mapping consistency (sanity check on canonical mapping)
    let mut errors = 0usize;
    for (sym, idx_opt) in sym_to_index.iter().enumerate() {
        if let Some(idx) = idx_opt {
            // Not checking trie internals here
        }
    }

    // Replicate the HuffmanTable::from_lengths algorithm directly and check for collisions
    let table_size = 1<<max_bits;
    let mut table2: Vec<u32> = Vec::new();
    table2.resize(table_size, 0xffffffffu32);
    let mut codes_arr2: Vec<u32> = Vec::new();
    codes_arr2.resize(max_bits + 1, 0u32);
    // recompute initial codes
    code = 0u32;
    for bits in 1..=max_bits { code = (code + counts[bits - 1] as u32) << 1; codes_arr2[bits] = code; }
    for (sym, &len) in lengths.iter().enumerate() {
        if len == 0 { continue; }
        let len_usz = len as usize;
        let c = codes_arr2[len_usz];
        codes_arr2[len_usz] += 1;
        let rev = {
            let mut v = c;
            let mut r = 0u32;
            for _ in 0..len_usz { r = (r << 1) | (v & 1); v >>= 1; }
            r
        };
        let start = (rev as usize) << (max_bits - len_usz);
        let end = start + (1usize << (max_bits - len_usz));
        for idx in start..end {
            if table2[idx] != 0xffffffff { println!("collision assigning sym {} at idx {}, currently has sym {}", sym, idx, table2[idx] >> 8); }
            table2[idx] = ((sym as u32) << 8) | (len as u32);
        }
    }

    // Print peek bits and decoded symbol for first several reads
    for _ in 0..10 {
        if let Some(peek) = br.peek_bits(9) {
            println!("peek9: {:#b} ({}) bytepos {}", peek, peek, br.byte_pos());
        }
        match lit_table.read_symbol(&mut br) {
            Ok(sym) => println!("sym: {}", sym),
            Err(e) => { println!("err: {:?} at bytepos {}", e, br.byte_pos()); break; }
        }
    }
}
