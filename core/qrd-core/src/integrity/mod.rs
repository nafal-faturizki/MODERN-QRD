pub fn crc32_bytes(bytes: &[u8]) -> u32 {
	crc32fast::hash(bytes)
}

pub fn crc32_matches(bytes: &[u8], expected: u32) -> bool {
	crc32_bytes(bytes) == expected
}
