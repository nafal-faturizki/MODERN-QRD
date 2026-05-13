use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
	#[error("file is too small: {file_size} bytes")]
	FileTooSmall { file_size: u64 },
	#[error("invalid footer version: {version}")]
	InvalidFooterVersion { version: u16 },
	#[error("unsupported major version: {major_version}")]
	UnsupportedMajorVersion { major_version: u16 },
	#[error("header row group count mismatch: header={header}, footer={footer}")]
	HeaderRowGroupCountMismatch { header: u32, footer: u32 },
	#[error("invalid file header magic")]
	InvalidMagic,
	#[error("header checksum mismatch")]
	HeaderChecksumMismatch,
	#[error("footer length is invalid: {footer_len} bytes for file size {file_size}")]
	InvalidFooterLength { footer_len: u32, file_size: u64 },
	#[error("footer body is too large: {body_len} bytes")]
	FooterTooLarge { body_len: usize },
	#[error("invalid row group header")]
	InvalidRowGroupHeader,
	#[error("invalid column chunk header")]
	InvalidColumnChunkHeader,
	#[error("unknown encryption id: {id:#04x}")]
	UnknownEncryption { id: u8 },
	#[error("schema id mismatch")]
	SchemaIdMismatch,
	#[error("unknown encoding id: {id:#04x}")]
	UnknownEncoding { id: u8 },
	#[error("unknown compression id: {id:#04x}")]
	UnknownCompression { id: u8 },
	#[error("authentication failed")]
	AuthenticationFailed,
	#[error("chunk checksum mismatch")]
	ChunkChecksumMismatch,
	#[error("footer checksum mismatch")]
	FooterChecksumMismatch,
	#[error("schema error: {0}")]
	Schema(#[from] crate::schema::SchemaError),
}
