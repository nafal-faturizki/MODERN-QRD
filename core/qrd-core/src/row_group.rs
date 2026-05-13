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

        let mut checksum_bytes = [0u8; 4];
        checksum_bytes.copy_from_slice(&bytes[8..12]);
        let stored_checksum = u32::from_le_bytes(checksum_bytes);
        let computed_checksum = integrity::crc32_bytes(&bytes[..8]);

        if stored_checksum != computed_checksum {
            return Err(Error::InvalidRowGroupHeader);
        }

        let mut row_count_bytes = [0u8; 4];
        row_count_bytes.copy_from_slice(&bytes[0..4]);
        let mut column_count_bytes = [0u8; 2];
        column_count_bytes.copy_from_slice(&bytes[4..6]);
        let mut rg_flags_bytes = [0u8; 2];
        rg_flags_bytes.copy_from_slice(&bytes[6..8]);

        Ok(Self {
            row_count: u32::from_le_bytes(row_count_bytes),
            column_count: u16::from_le_bytes(column_count_bytes),
            rg_flags: u16::from_le_bytes(rg_flags_bytes),
        })
    }
}
