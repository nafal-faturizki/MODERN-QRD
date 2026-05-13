use qrd_core::encoding;

#[test]
fn plain_roundtrip() {
    let data = b"hello world";
    let encoded = encoding::plain::encode(data).unwrap();
    let decoded = encoding::plain::decode(&encoded).unwrap();
    assert_eq!(&decoded[..], data);
}

#[test]
fn rle_simple() {
    let data = vec![1u8, 1, 1, 2, 2];
    let encoded = encoding::rle::encode(&data).unwrap();
    let decoded = encoding::rle::decode(&encoded).unwrap();
    assert_eq!(&decoded[..], &data[..]);
}

#[test]
fn bit_packed_encode_simple() {
    let data = vec![0u8, 1, 2, 3];
    let encoded = encoding::bit_packed::encode(&data).unwrap();
    assert_eq!(u32::from_le_bytes(encoded[0..4].try_into().unwrap()), data.len() as u32);
    assert_eq!(encoded[4], 2); // bit_width = 2 (max value 3)
    let decoded = encoding::bit_packed::decode(&encoded).unwrap();
    assert_eq!(&decoded[..], &data[..]);
}

#[test]
fn delta_binary_simple() {
    let data = vec![100u32, 102, 105];
    let mut bytes = Vec::new();
    for v in data {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    let encoded = encoding::delta_binary::encode(&bytes).unwrap();
    let decoded = encoding::delta_binary::decode(&encoded).unwrap();
    assert_eq!(decoded, bytes);
}

#[test]
fn delta_byte_array_roundtrip() {
    let data = b"mississippi";
    let encoded = encoding::delta_byte_array::encode(data).unwrap();
    let decoded = encoding::delta_byte_array::decode(&encoded).unwrap();
    assert_eq!(&decoded[..], data);
}

#[test]
fn byte_stream_split_roundtrip() {
    let data = b"abcdefghijklmnop";
    let encoded = encoding::byte_stream_split::encode(data).unwrap();
    let decoded = encoding::byte_stream_split::decode(&encoded).unwrap();
    assert_eq!(&decoded[..], data);
}

#[test]
fn dictionary_rle_roundtrip() {
    let data = b"aaabbbccccddddaaa";
    let encoded = encoding::dictionary_rle::encode(data).unwrap();
    let decoded = encoding::dictionary_rle::decode(&encoded).unwrap();
    assert_eq!(&decoded[..], data);
}
