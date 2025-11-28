use wee_alloc::WeeAlloc;

#[global_allocator]
static ALLOC: WeeAlloc = WeeAlloc::INIT;

use no_inflate::inflate_zlib;
use flate2::{Compression, write::ZlibEncoder};
use std::io::Write;

#[test]
fn global_allocator_integration() {
    // Generate compressed data
    let input = b"example data to compress with global allocator";
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(input).unwrap();
    let compressed = enc.finish().unwrap();
    // Use the library to decompress
    let decompressed = inflate_zlib(&compressed).expect("decompress");
    assert_eq!(decompressed, input);
}
