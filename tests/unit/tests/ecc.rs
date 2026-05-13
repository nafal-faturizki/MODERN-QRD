use qrd_core::ecc::{self, EccConfig};
use qrd_core::Error;

#[test]
fn ecc_config_validation() {
    assert!(EccConfig::new(10, 5).is_ok());
    assert!(EccConfig::new(5, 10).is_err()); // n must be > k
    assert!(EccConfig::new(5, 5).is_err()); // n must be > k
}

#[test]
fn ecc_roundtrip() {
    let config = EccConfig::new(10, 7).unwrap();
    let data: Vec<Vec<u8>> = vec![vec![1, 2, 3, 4]; 7];
    let data_refs: Vec<&[u8]> = data.iter().map(|v| v.as_slice()).collect();

    let parity = ecc::encode(&config, &data_refs).unwrap();
    assert_eq!(parity.len(), 3); // 10 - 7 = 3 parity chunks
}

#[test]
fn ecc_decode_recovers_missing_data_chunk() {
    let config = EccConfig::new(5, 3).unwrap();
    let data: Vec<Vec<u8>> = vec![vec![10, 11, 12, 13], vec![20, 21, 22, 23], vec![30, 31, 32, 33]];
    let data_refs: Vec<&[u8]> = data.iter().map(|v| v.as_slice()).collect();
    let parity = ecc::encode(&config, &data_refs).unwrap();

    let mut shards: Vec<Option<Vec<u8>>> = Vec::new();
    shards.push(Some(data[0].clone()));
    shards.push(None); // missing data shard to recover
    shards.push(Some(data[2].clone()));
    shards.push(Some(parity[0].clone()));
    shards.push(Some(parity[1].clone()));

    let recovered = ecc::decode(&config, &shards).unwrap();
    assert_eq!(recovered, data);
}

#[test]
fn ecc_decode_rejects_inconsistent_chunk_sizes() {
    let config = EccConfig::new(5, 3).unwrap();
    let shards: Vec<Option<Vec<u8>>> = vec![
        Some(vec![1, 2, 3, 4]),
        Some(vec![5, 6, 7]), // inconsistent
        Some(vec![8, 9, 10, 11]),
        Some(vec![0, 0, 0, 0]),
        Some(vec![0, 0, 0, 0]),
    ];

    let err = ecc::decode(&config, &shards).unwrap_err();
    assert!(matches!(err, Error::EccShardLengthMismatch));
}

#[test]
fn ecc_config_parity_count() {
    let config = EccConfig::new(10, 7).unwrap();
    assert_eq!(config.parity_count(), 3);
}
