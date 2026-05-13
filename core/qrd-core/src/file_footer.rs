use crate::{integrity, Error, Result};

pub const FOOTER_LENGTH_FIELD_SIZE: usize = 4;

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

pub fn decode_footer_body(file_bytes: &[u8]) -> Result<Vec<u8>> {
    if file_bytes.len() < FOOTER_LENGTH_FIELD_SIZE {
        return Err(Error::FileTooSmall {
            file_size: file_bytes.len() as u64,
        });
    }

    let mut footer_length_bytes = [0u8; 4];
    footer_length_bytes.copy_from_slice(&file_bytes[file_bytes.len() - 4..]);
    let footer_length = u32::from_le_bytes(footer_length_bytes) as usize;

    if footer_length < FOOTER_LENGTH_FIELD_SIZE {
        return Err(Error::InvalidFooterLength {
            footer_len: footer_length as u32,
            file_size: file_bytes.len() as u64,
        });
    }

    if footer_length + FOOTER_LENGTH_FIELD_SIZE > file_bytes.len() {
        return Err(Error::InvalidFooterLength {
            footer_len: footer_length as u32,
            file_size: file_bytes.len() as u64,
        });
    }

    let footer_start = file_bytes.len() - FOOTER_LENGTH_FIELD_SIZE - footer_length;
    let footer_content = &file_bytes[footer_start..footer_start + footer_length];

    if footer_content.len() < FOOTER_LENGTH_FIELD_SIZE {
        return Err(Error::InvalidFooterLength {
            footer_len: footer_length as u32,
            file_size: file_bytes.len() as u64,
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
