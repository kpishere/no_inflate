use no_inflate::inflate_zlib;

#[test]
fn roundtrip_simple() {
    let input = b"The quick brown fox jumps over the lazy dog";
    // Compress with flate2 zlib
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(input).unwrap();
    let compressed = enc.finish().unwrap();

    let decompressed = inflate_zlib(&compressed).expect("decompress");
    assert_eq!(decompressed, input);
}

#[test]
fn roundtrip_large() {
    let mut data = Vec::new();
    for i in 0..1000 {
        data.extend_from_slice(b"Hello WORLD '123456' ABCDEFGHIJKLMNOPQRSTUVWXYZ\n");
    }
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&data).unwrap();
    let compressed = enc.finish().unwrap();

    let decompressed = inflate_zlib(&compressed).expect("decompress");
    assert_eq!(decompressed, data);
}
