use crate::{integrity, Error, Result};

pub const FILE_HEADER_SIZE: usize = 32;
pub const FILE_HEADER_MAGIC: [u8; 4] = [0x51, 0x52, 0x44, 0x01];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHeader {
    pub major_version: u16,
    pub minor_version: u16,
    pub schema_id: [u8; 8],
    pub flags: u32,
    pub row_group_count: u32,
    pub created_at_sec: u32,
}

impl FileHeader {
    pub fn new(
        major_version: u16,
        minor_version: u16,
        schema_id: [u8; 8],
        flags: u32,
        row_group_count: u32,
        created_at_sec: u32,
    ) -> Self {
        Self {
            major_version,
            minor_version,
            schema_id,
            flags,
            row_group_count,
            created_at_sec,
        }
    }

    pub fn to_bytes(&self) -> [u8; FILE_HEADER_SIZE] {
        let mut bytes = [0u8; FILE_HEADER_SIZE];

        bytes[0..4].copy_from_slice(&FILE_HEADER_MAGIC);
        bytes[4..6].copy_from_slice(&self.major_version.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.minor_version.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.schema_id);
        bytes[16..20].copy_from_slice(&self.flags.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.row_group_count.to_le_bytes());
        bytes[24..28].copy_from_slice(&self.created_at_sec.to_le_bytes());

        let header_crc32 = integrity::crc32_bytes(&bytes[..28]);
        bytes[28..32].copy_from_slice(&header_crc32.to_le_bytes());

        bytes
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < FILE_HEADER_SIZE {
            return Err(Error::FileTooSmall {
                file_size: bytes.len() as u64,
            });
        }

        if bytes[0..4] != FILE_HEADER_MAGIC {
            return Err(Error::InvalidMagic);
        }

        let mut stored_crc32 = [0u8; 4];
        stored_crc32.copy_from_slice(&bytes[28..32]);
        let stored_crc32 = u32::from_le_bytes(stored_crc32);
        let computed_crc32 = integrity::crc32_bytes(&bytes[..28]);

        if stored_crc32 != computed_crc32 {
            return Err(Error::HeaderChecksumMismatch);
        }

        let mut schema_id = [0u8; 8];
        schema_id.copy_from_slice(&bytes[8..16]);

        Ok(Self {
            major_version: u16::from_le_bytes([bytes[4], bytes[5]]),
            minor_version: u16::from_le_bytes([bytes[6], bytes[7]]),
            schema_id,
            flags: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            row_group_count: u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            created_at_sec: u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
        })
    }

    pub fn validate_major_version(&self, supported_major_version: u16) -> Result<()> {
        if self.major_version != supported_major_version {
            return Err(Error::UnsupportedMajorVersion {
                major_version: self.major_version,
            });
        }

        Ok(())
    }
}
