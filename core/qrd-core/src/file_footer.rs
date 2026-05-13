use crate::file_header::FILE_HEADER_SIZE;
use crate::schema::Schema;
use crate::{integrity, Error, Result};

pub const FOOTER_LENGTH_FIELD_SIZE: usize = 4;
pub const FILE_FLAG_ENCRYPTED: u32 = 0x0000_0001;
pub const FILE_FLAG_SCHEMA_SIGNED: u32 = 0x0000_0008;
pub const FOOTER_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FooterRowGroupEntry {
    pub byte_offset: u64,
    pub row_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FooterEncryptionMetadata {
    pub key_derivation_algo: u8,
    pub kdf_params: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FooterSchemaSignature {
    pub sig_algo: u8,
    pub signature: [u8; 64],
    pub public_key: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FooterContent {
    pub footer_version: u16,
    pub schema: Schema,
    pub row_groups: Vec<FooterRowGroupEntry>,
    pub statistics_flag: u8,
    pub statistics_bytes: Vec<u8>,
    pub encryption_metadata: Option<FooterEncryptionMetadata>,
    pub schema_signature: Option<FooterSchemaSignature>,
    pub file_metadata: Vec<u8>,
}

impl FooterContent {
    pub fn new(schema: Schema) -> Self {
        Self {
            footer_version: FOOTER_VERSION,
            schema,
            row_groups: Vec::new(),
            statistics_flag: 0x00,
            statistics_bytes: Vec::new(),
            encryption_metadata: None,
            schema_signature: None,
            file_metadata: Vec::new(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.footer_version.to_le_bytes());
        out.extend_from_slice(&self.schema.serialize_footer_schema_section()?);

        let row_group_count = u32::try_from(self.row_groups.len()).map_err(|_| Error::FooterTooLarge {
            body_len: self.row_groups.len(),
        })?;
        out.extend_from_slice(&row_group_count.to_le_bytes());

        for row_group in &self.row_groups {
            out.extend_from_slice(&row_group.byte_offset.to_le_bytes());
            out.extend_from_slice(&row_group.row_count.to_le_bytes());
        }

        out.push(self.statistics_flag);
        let statistics_len = u32::try_from(self.statistics_bytes.len()).map_err(|_| Error::FooterTooLarge {
            body_len: self.statistics_bytes.len(),
        })?;
        out.extend_from_slice(&statistics_len.to_le_bytes());
        out.extend_from_slice(&self.statistics_bytes);

        if let Some(encryption_metadata) = &self.encryption_metadata {
            out.push(encryption_metadata.key_derivation_algo);
            let kdf_params_len = u16::try_from(encryption_metadata.kdf_params.len()).map_err(|_| Error::FooterTooLarge {
                body_len: encryption_metadata.kdf_params.len(),
            })?;
            out.extend_from_slice(&kdf_params_len.to_le_bytes());
            out.extend_from_slice(&encryption_metadata.kdf_params);
        }

        if let Some(signature) = &self.schema_signature {
            out.push(signature.sig_algo);
            out.extend_from_slice(&signature.signature);
            out.extend_from_slice(&signature.public_key);
        }

        let metadata_len = u32::try_from(self.file_metadata.len()).map_err(|_| Error::FooterTooLarge {
            body_len: self.file_metadata.len(),
        })?;
        out.extend_from_slice(&metadata_len.to_le_bytes());
        out.extend_from_slice(&self.file_metadata);

        Ok(out)
    }

    pub fn parse(body: &[u8], header_flags: u32) -> Result<Self> {
        let mut cursor = 0usize;

        let footer_version = read_u16(body, &mut cursor)?;
        if footer_version != FOOTER_VERSION {
            return Err(Error::InvalidFooterVersion {
                version: footer_version,
            });
        }

        let (schema, consumed) = Schema::parse_footer_schema_section(&body[cursor..])?;
        cursor = cursor
            .checked_add(consumed)
            .ok_or(Error::InvalidFooterLength {
                footer_len: body.len() as u32,
                file_size: body.len() as u64,
            })?;

        let row_group_count = read_u32(body, &mut cursor)? as usize;
        let mut row_groups = Vec::with_capacity(row_group_count);
        for _ in 0..row_group_count {
            let byte_offset = read_u64(body, &mut cursor)?;
            let row_count = read_u32(body, &mut cursor)?;
            row_groups.push(FooterRowGroupEntry {
                byte_offset,
                row_count,
            });
        }

        let statistics_flag = read_u8(body, &mut cursor)?;
        let statistics_len = read_u32(body, &mut cursor)? as usize;
        let statistics_bytes = read_vec(body, &mut cursor, statistics_len)?.to_vec();

        let encryption_metadata = if header_flags & FILE_FLAG_ENCRYPTED != 0 {
            let key_derivation_algo = read_u8(body, &mut cursor)?;
            let kdf_params_len = read_u16(body, &mut cursor)? as usize;
            let kdf_params = read_vec(body, &mut cursor, kdf_params_len)?.to_vec();
            Some(FooterEncryptionMetadata {
                key_derivation_algo,
                kdf_params,
            })
        } else {
            None
        };

        let schema_signature = if header_flags & FILE_FLAG_SCHEMA_SIGNED != 0 {
            let sig_algo = read_u8(body, &mut cursor)?;
            let signature = read_array::<64>(body, &mut cursor)?;
            let public_key = read_array::<32>(body, &mut cursor)?;
            Some(FooterSchemaSignature {
                sig_algo,
                signature,
                public_key,
            })
        } else {
            None
        };

        let file_metadata_len = read_u32(body, &mut cursor)? as usize;
        let file_metadata = read_vec(body, &mut cursor, file_metadata_len)?.to_vec();

        if cursor != body.len() {
            return Err(Error::InvalidFooterLength {
                footer_len: body.len() as u32,
                file_size: body.len() as u64,
            });
        }

        Ok(Self {
            footer_version,
            schema,
            row_groups,
            statistics_flag,
            statistics_bytes,
            encryption_metadata,
            schema_signature,
            file_metadata,
        })
    }

    pub fn row_group_count(&self) -> u32 {
        self.row_groups.len() as u32
    }

    pub fn total_rows(&self) -> u64 {
        self.row_groups.iter().map(|entry| entry.row_count as u64).sum()
    }
}

pub fn encode_footer_envelope(body_without_checksum: &[u8]) -> Result<Vec<u8>> {
    let footer_content_length = body_without_checksum
        .len()
        .checked_add(FOOTER_LENGTH_FIELD_SIZE)
        .ok_or(Error::FooterTooLarge {
            body_len: body_without_checksum.len(),
        })?;

    let footer_length = u32::try_from(footer_content_length).map_err(|_| Error::FooterTooLarge {
        body_len: body_without_checksum.len(),
    })?;

    let checksum = integrity::crc32_bytes(body_without_checksum);

    let mut out = Vec::with_capacity(footer_content_length + FOOTER_LENGTH_FIELD_SIZE);
    out.extend_from_slice(body_without_checksum);
    out.extend_from_slice(&checksum.to_le_bytes());
    out.extend_from_slice(&footer_length.to_le_bytes());
    Ok(out)
}

pub fn decode_footer_envelope(envelope: &[u8]) -> Result<Vec<u8>> {
    if envelope.len() < FOOTER_LENGTH_FIELD_SIZE {
        return Err(Error::FileTooSmall {
            file_size: envelope.len() as u64,
        });
    }

    let mut footer_length_bytes = [0u8; 4];
    footer_length_bytes.copy_from_slice(&envelope[envelope.len() - 4..]);
    let footer_length = u32::from_le_bytes(footer_length_bytes) as usize;

    if footer_length < FOOTER_LENGTH_FIELD_SIZE || footer_length + FOOTER_LENGTH_FIELD_SIZE != envelope.len() {
        return Err(Error::InvalidFooterLength {
            footer_len: footer_length as u32,
            file_size: envelope.len() as u64,
        });
    }

    let footer_content = &envelope[..envelope.len() - FOOTER_LENGTH_FIELD_SIZE];

    if footer_content.len() < FOOTER_LENGTH_FIELD_SIZE {
        return Err(Error::InvalidFooterLength {
            footer_len: footer_length as u32,
            file_size: envelope.len() as u64,
        });
    }

    let body_len = footer_content.len() - FOOTER_LENGTH_FIELD_SIZE;
    let (body, stored_checksum_bytes) = footer_content.split_at(body_len);

    let mut stored_checksum = [0u8; 4];
    stored_checksum.copy_from_slice(stored_checksum_bytes);
    let stored_checksum = u32::from_le_bytes(stored_checksum);
    let computed_checksum = integrity::crc32_bytes(body);

    if stored_checksum != computed_checksum {
        return Err(Error::FooterChecksumMismatch);
    }

    Ok(body.to_vec())
}

pub fn decode_footer_body(file_bytes: &[u8]) -> Result<Vec<u8>> {
    if file_bytes.len() < FOOTER_LENGTH_FIELD_SIZE {
        return Err(Error::FileTooSmall {
            file_size: file_bytes.len() as u64,
        });
    }

    let mut footer_length_bytes = [0u8; 4];
    footer_length_bytes.copy_from_slice(&file_bytes[file_bytes.len() - 4..]);
    let footer_length = u32::from_le_bytes(footer_length_bytes) as usize;

    if footer_length < FOOTER_LENGTH_FIELD_SIZE || footer_length + FOOTER_LENGTH_FIELD_SIZE > file_bytes.len() {
        return Err(Error::InvalidFooterLength {
            footer_len: footer_length as u32,
            file_size: file_bytes.len() as u64,
        });
    }

    let footer_start = file_bytes.len() - FOOTER_LENGTH_FIELD_SIZE - footer_length;
    decode_footer_envelope(&file_bytes[footer_start..])
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8> {
    let slice = read_vec(bytes, cursor, 1)?;
    Ok(slice[0])
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16> {
    let slice = read_vec(bytes, cursor, 2)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32> {
    let slice = read_vec(bytes, cursor, 4)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64> {
    let slice = read_vec(bytes, cursor, 8)?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn read_array<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Result<[u8; N]> {
    let slice = read_vec(bytes, cursor, N)?;
    let mut out = [0u8; N];
    out.copy_from_slice(slice);
    Ok(out)
}

fn read_vec<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8]> {
    let end = cursor.checked_add(len).ok_or(Error::InvalidFooterLength {
        footer_len: bytes.len() as u32,
        file_size: bytes.len() as u64,
    })?;
    let slice = bytes.get(*cursor..end).ok_or(Error::InvalidFooterLength {
        footer_len: bytes.len() as u32,
        file_size: bytes.len() as u64,
    })?;
    *cursor = end;
    Ok(slice)
}

#[allow(dead_code)]
fn _footer_size_floor() -> usize {
    FILE_HEADER_SIZE
}