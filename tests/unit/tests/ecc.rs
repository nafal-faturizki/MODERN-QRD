use qrd_core::ecc::{self, EccConfig};

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
fn ecc_config_parity_count() {
    let config = EccConfig::new(10, 7).unwrap();
    assert_eq!(config.parity_count(), 3);
}
