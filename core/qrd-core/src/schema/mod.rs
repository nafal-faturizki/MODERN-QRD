use sha2::{Digest, Sha256};
use std::convert::TryFrom;
use std::str;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    schema_version: u16,
    fields: Vec<SchemaField>,
}

impl Schema {
    pub fn builder() -> SchemaBuilder {
        SchemaBuilder::new()
    }

    pub fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub fn fields(&self) -> &[SchemaField] {
        &self.fields
    }

    pub fn serialize_footer_schema_section(&self) -> Result<Vec<u8>, SchemaError> {
        let payload = self.serialize_footer_schema_payload()?;
        let payload_len = u32::try_from(payload.len()).map_err(|_| SchemaError::SectionTooLarge)?;

        let mut out = Vec::with_capacity(4 + payload.len());
        out.extend_from_slice(&payload_len.to_le_bytes());
        out.extend_from_slice(&payload);
        Ok(out)
    }

    pub fn schema_fingerprint(&self) -> Result<[u8; 32], SchemaError> {
        let payload = self.serialize_footer_schema_payload()?;
        let digest = Sha256::digest(payload);
        Ok(digest.into())
    }

    pub fn schema_id(&self) -> Result<[u8; 8], SchemaError> {
        let fingerprint = self.schema_fingerprint()?;
        let mut schema_id = [0u8; 8];
        schema_id.copy_from_slice(&fingerprint[..8]);
        Ok(schema_id)
    }

    pub fn parse_footer_schema_section(bytes: &[u8]) -> Result<(Self, usize), SchemaError> {
        if bytes.len() < 4 {
            return Err(SchemaError::TruncatedSection);
        }

        let mut payload_len_bytes = [0u8; 4];
        payload_len_bytes.copy_from_slice(&bytes[0..4]);
        let payload_len = u32::from_le_bytes(payload_len_bytes) as usize;
        let total_len = 4usize
            .checked_add(payload_len)
            .ok_or(SchemaError::SectionTooLarge)?;

        if bytes.len() < total_len {
            return Err(SchemaError::TruncatedSection);
        }

        let payload = &bytes[4..total_len];
        let mut cursor = 0usize;

        let schema_version = read_u16(payload, &mut cursor)?;
        let field_count = read_u16(payload, &mut cursor)? as usize;

        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            fields.push(parse_schema_field(payload, &mut cursor)?);
        }

        if cursor != payload.len() {
            return Err(SchemaError::InvalidFooterSchemaPayload);
        }

        Ok((Schema { schema_version, fields }, total_len))
    }

    fn serialize_footer_schema_payload(&self) -> Result<Vec<u8>, SchemaError> {
        let field_count = u16::try_from(self.fields.len()).map_err(|_| SchemaError::TooManyFields)?;

        let mut out = Vec::new();
        out.extend_from_slice(&self.schema_version.to_le_bytes());
        out.extend_from_slice(&field_count.to_le_bytes());

        for field in &self.fields {
            field.serialize_into(&mut out)?;
        }

        Ok(out)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaBuilder {
    schema_version: u16,
    fields: Vec<SchemaField>,
}

impl SchemaBuilder {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            fields: Vec::new(),
        }
    }

    pub fn schema_version(mut self, schema_version: u16) -> Self {
        self.schema_version = schema_version;
        self
    }

    pub fn field(mut self, field: SchemaField) -> Self {
        self.fields.push(field);
        self
    }

    pub fn fields(mut self, fields: impl IntoIterator<Item = SchemaField>) -> Self {
        self.fields.extend(fields);
        self
    }

    pub fn build(self) -> Result<Schema, SchemaError> {
        Ok(Schema {
            schema_version: self.schema_version,
            fields: self.fields,
        })
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaField {
    pub name: String,
    pub logical_type_id: LogicalTypeId,
    pub nullability: Nullability,
    pub encoding_hint: EncodingHint,
    pub compression_hint: CompressionHint,
    pub encryption_id: EncryptionId,
    pub metadata: Vec<SchemaMetadataEntry>,
}

impl SchemaField {
    pub fn new(
        name: impl Into<String>,
        logical_type_id: LogicalTypeId,
        nullability: Nullability,
    ) -> Self {
        Self {
            name: name.into(),
            logical_type_id,
            nullability,
            encoding_hint: EncodingHint::Plain,
            compression_hint: CompressionHint::None,
            encryption_id: EncryptionId::None,
            metadata: Vec::new(),
        }
    }

    pub fn with_encoding_hint(mut self, encoding_hint: EncodingHint) -> Self {
        self.encoding_hint = encoding_hint;
        self
    }

    pub fn with_compression_hint(mut self, compression_hint: CompressionHint) -> Self {
        self.compression_hint = compression_hint;
        self
    }

    pub fn with_encryption_id(mut self, encryption_id: EncryptionId) -> Self {
        self.encryption_id = encryption_id;
        self
    }

    pub fn with_metadata(mut self, metadata: impl IntoIterator<Item = SchemaMetadataEntry>) -> Self {
        self.metadata.extend(metadata);
        self
    }

    fn serialize_into(&self, out: &mut Vec<u8>) -> Result<(), SchemaError> {
        let name_bytes = self.name.as_bytes();
        let name_len = u16::try_from(name_bytes.len()).map_err(|_| SchemaError::FieldNameTooLong)?;
        let metadata_count = u16::try_from(self.metadata.len()).map_err(|_| SchemaError::TooManyMetadataEntries)?;

        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(name_bytes);
        out.push(self.logical_type_id as u8);
        out.push(self.nullability as u8);
        out.push(self.encoding_hint as u8);
        out.push(self.compression_hint as u8);
        out.push(self.encryption_id as u8);
        out.extend_from_slice(&metadata_count.to_le_bytes());

        for entry in &self.metadata {
            entry.serialize_into(out)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaMetadataEntry {
    pub key: String,
    pub value: String,
}

impl SchemaMetadataEntry {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    fn serialize_into(&self, out: &mut Vec<u8>) -> Result<(), SchemaError> {
        let key_bytes = self.key.as_bytes();
        let value_bytes = self.value.as_bytes();
        let key_len = u16::try_from(key_bytes.len()).map_err(|_| SchemaError::MetadataKeyTooLong)?;
        let value_len = u16::try_from(value_bytes.len()).map_err(|_| SchemaError::MetadataValueTooLong)?;

        out.extend_from_slice(&key_len.to_le_bytes());
        out.extend_from_slice(key_bytes);
        out.extend_from_slice(&value_len.to_le_bytes());
        out.extend_from_slice(value_bytes);
        Ok(())
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalTypeId {
    Boolean = 0x01,
    Int8 = 0x02,
    Int16 = 0x03,
    Int32 = 0x04,
    Int64 = 0x05,
    UInt8 = 0x06,
    UInt16 = 0x07,
    UInt32 = 0x08,
    UInt64 = 0x09,
    Float32 = 0x0A,
    Float64 = 0x0B,
    Timestamp = 0x10,
    Date = 0x11,
    Time = 0x12,
    Duration = 0x13,
    Utf8String = 0x20,
    Enum = 0x21,
    Uuid = 0x22,
    Blob = 0x23,
    Decimal = 0x24,
    Struct = 0x30,
    Array = 0x31,
    Map = 0x32,
    Any = 0xFF,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nullability {
    Required = 0x00,
    Optional = 0x01,
    Repeated = 0x02,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingHint {
    Plain = 0x00,
    Rle = 0x01,
    BitPacked = 0x02,
    DeltaBinary = 0x03,
    DeltaByteArray = 0x04,
    ByteStreamSplit = 0x05,
    DictionaryRle = 0x06,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionHint {
    None = 0x00,
    Zstd = 0x01,
    Lz4Frame = 0x02,
    Snappy = 0x03,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionId {
    None = 0x00,
    Aes256Gcm = 0x01,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SchemaError {
    #[error("schema contains too many fields to fit into footer encoding")]
    TooManyFields,
    #[error("schema section exceeds the U32 length limit")]
    SectionTooLarge,
    #[error("schema footer section is truncated")]
    TruncatedSection,
    #[error("schema footer payload is invalid")]
    InvalidFooterSchemaPayload,
    #[error("field name exceeds the U16 length limit")]
    FieldNameTooLong,
    #[error("field contains too many metadata entries to fit into footer encoding")]
    TooManyMetadataEntries,
    #[error("metadata key exceeds the U16 length limit")]
    MetadataKeyTooLong,
    #[error("metadata value exceeds the U16 length limit")]
    MetadataValueTooLong,
    #[error("schema footer contains invalid utf-8")]
    InvalidUtf8,
    #[error("unknown logical type id: {id:#04x}")]
    UnknownLogicalTypeId { id: u8 },
    #[error("unknown nullability id: {id:#04x}")]
    UnknownNullabilityId { id: u8 },
    #[error("unknown encoding hint id: {id:#04x}")]
    UnknownEncodingHintId { id: u8 },
    #[error("unknown compression hint id: {id:#04x}")]
    UnknownCompressionHintId { id: u8 },
    #[error("unknown encryption id: {id:#04x}")]
    UnknownEncryptionId { id: u8 },
}

impl TryFrom<u8> for LogicalTypeId {
    type Error = SchemaError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Boolean),
            0x02 => Ok(Self::Int8),
            0x03 => Ok(Self::Int16),
            0x04 => Ok(Self::Int32),
            0x05 => Ok(Self::Int64),
            0x06 => Ok(Self::UInt8),
            0x07 => Ok(Self::UInt16),
            0x08 => Ok(Self::UInt32),
            0x09 => Ok(Self::UInt64),
            0x0A => Ok(Self::Float32),
            0x0B => Ok(Self::Float64),
            0x10 => Ok(Self::Timestamp),
            0x11 => Ok(Self::Date),
            0x12 => Ok(Self::Time),
            0x13 => Ok(Self::Duration),
            0x20 => Ok(Self::Utf8String),
            0x21 => Ok(Self::Enum),
            0x22 => Ok(Self::Uuid),
            0x23 => Ok(Self::Blob),
            0x24 => Ok(Self::Decimal),
            0x30 => Ok(Self::Struct),
            0x31 => Ok(Self::Array),
            0x32 => Ok(Self::Map),
            0xFF => Ok(Self::Any),
            id => Err(SchemaError::UnknownLogicalTypeId { id }),
        }
    }
}

impl TryFrom<u8> for Nullability {
    type Error = SchemaError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Required),
            0x01 => Ok(Self::Optional),
            0x02 => Ok(Self::Repeated),
            id => Err(SchemaError::UnknownNullabilityId { id }),
        }
    }
}

impl TryFrom<u8> for EncodingHint {
    type Error = SchemaError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Plain),
            0x01 => Ok(Self::Rle),
            0x02 => Ok(Self::BitPacked),
            0x03 => Ok(Self::DeltaBinary),
            0x04 => Ok(Self::DeltaByteArray),
            0x05 => Ok(Self::ByteStreamSplit),
            0x06 => Ok(Self::DictionaryRle),
            id => Err(SchemaError::UnknownEncodingHintId { id }),
        }
    }
}

impl TryFrom<u8> for CompressionHint {
    type Error = SchemaError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::None),
            0x01 => Ok(Self::Zstd),
            0x02 => Ok(Self::Lz4Frame),
            0x03 => Ok(Self::Snappy),
            id => Err(SchemaError::UnknownCompressionHintId { id }),
        }
    }
}

impl TryFrom<u8> for EncryptionId {
    type Error = SchemaError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::None),
            0x01 => Ok(Self::Aes256Gcm),
            id => Err(SchemaError::UnknownEncryptionId { id }),
        }
    }
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, SchemaError> {
    let end = cursor.checked_add(2).ok_or(SchemaError::TruncatedSection)?;
    let slice = bytes.get(*cursor..end).ok_or(SchemaError::TruncatedSection)?;
    *cursor = end;
    let mut out = [0u8; 2];
    out.copy_from_slice(slice);
    Ok(u16::from_le_bytes(out))
}

fn read_bytes<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], SchemaError> {
    let end = cursor.checked_add(len).ok_or(SchemaError::TruncatedSection)?;
    let slice = bytes.get(*cursor..end).ok_or(SchemaError::TruncatedSection)?;
    *cursor = end;
    Ok(slice)
}

fn parse_schema_field(bytes: &[u8], cursor: &mut usize) -> Result<SchemaField, SchemaError> {
    let name_len = read_u16(bytes, cursor)? as usize;
    let name_bytes = read_bytes(bytes, cursor, name_len)?;
    let name = str::from_utf8(name_bytes).map_err(|_| SchemaError::InvalidUtf8)?.to_string();

    let logical_type_id = LogicalTypeId::try_from(read_bytes(bytes, cursor, 1)?[0])?;
    let nullability = Nullability::try_from(read_bytes(bytes, cursor, 1)?[0])?;
    let encoding_hint = EncodingHint::try_from(read_bytes(bytes, cursor, 1)?[0])?;
    let compression_hint = CompressionHint::try_from(read_bytes(bytes, cursor, 1)?[0])?;
    let encryption_id = EncryptionId::try_from(read_bytes(bytes, cursor, 1)?[0])?;

    let metadata_count = read_u16(bytes, cursor)? as usize;
    let mut metadata = Vec::with_capacity(metadata_count);

    for _ in 0..metadata_count {
        let key_len = read_u16(bytes, cursor)? as usize;
        let key = str::from_utf8(read_bytes(bytes, cursor, key_len)?).map_err(|_| SchemaError::InvalidUtf8)?.to_string();

        let value_len = read_u16(bytes, cursor)? as usize;
        let value = str::from_utf8(read_bytes(bytes, cursor, value_len)?).map_err(|_| SchemaError::InvalidUtf8)?.to_string();

        metadata.push(SchemaMetadataEntry { key, value });
    }

    Ok(SchemaField {
        name,
        logical_type_id,
        nullability,
        encoding_hint,
        compression_hint,
        encryption_id,
        metadata,
    })
}
