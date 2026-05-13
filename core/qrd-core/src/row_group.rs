use crate::{integrity, Error, Result};

pub const ROW_GROUP_HEADER_SIZE: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowGroupHeader {
    pub row_count: u32,
    pub column_count: u16,
    pub rg_flags: u16,
}

impl RowGroupHeader {
    pub fn new(row_count: u32, column_count: u16, rg_flags: u16) -> Self {
        Self {
            row_count,
            column_count,
            rg_flags,
        }
    }

    pub fn to_bytes(&self) -> [u8; ROW_GROUP_HEADER_SIZE] {
        let mut bytes = [0u8; ROW_GROUP_HEADER_SIZE];

        bytes[0..4].copy_from_slice(&self.row_count.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.column_count.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.rg_flags.to_le_bytes());

        let checksum = integrity::crc32_bytes(&bytes[..8]);
        bytes[8..12].copy_from_slice(&checksum.to_le_bytes());

        bytes
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < ROW_GROUP_HEADER_SIZE {
            return Err(Error::FileTooSmall {
                file_size: bytes.len() as u64,
            });
        }

        let stored_checksum = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let computed_checksum = integrity::crc32_bytes(&bytes[..8]);

        if stored_checksum != computed_checksum {
            return Err(Error::InvalidRowGroupHeader);
        }

        Ok(Self {
            row_count: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            column_count: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
            rg_flags: u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
        })
    }
}
