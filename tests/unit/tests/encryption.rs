use qrd_core::encryption::Cipher;

#[test]
fn cipher_roundtrip() {
    let master_key: [u8; 32] = [42u8; 32];
    let plaintext = b"Hello, World!";
    let nonce = Cipher::generate_nonce();

    let cipher = Cipher::new(&master_key);
    let ciphertext = cipher.encrypt(&nonce, plaintext).unwrap();
    let decrypted = cipher.decrypt(&nonce, &ciphertext).unwrap();

    assert_eq!(&decrypted[..], plaintext);
}

#[test]
fn encrypt_decrypt_blob() {
    let master_key: [u8; 32] = [99u8; 32];
    let plaintext = b"Test data for encryption";

    let blob = qrd_core::encryption::encrypt(&master_key, plaintext).unwrap();
    assert!(blob.len() > 12); // nonce + ciphertext + auth_tag

    let decrypted = qrd_core::encryption::decrypt(&master_key, &blob).unwrap();
    assert_eq!(&decrypted[..], plaintext);
}

#[test]
fn wrong_master_key_fails() {
    let master_key: [u8; 32] = [42u8; 32];
    let wrong_key: [u8; 32] = [99u8; 32];
    let plaintext = b"Secret data";

    let blob = qrd_core::encryption::encrypt(&master_key, plaintext).unwrap();
    let result = qrd_core::encryption::decrypt(&wrong_key, &blob);

    assert!(result.is_err());
}

#[test]
fn hkdf_derivation() {
    let master_key: [u8; 32] = [1u8; 32];
    let salt: [u8; 32] = [2u8; 32];
    let info = b"column_key_1";

    let key1 = Cipher::derive_key(&master_key, Some(&salt), info).unwrap();
    let key2 = Cipher::derive_key(&master_key, Some(&salt), info).unwrap();

    // Same inputs should produce same key
    assert_eq!(&key1[..], &key2[..]);
}
