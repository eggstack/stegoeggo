//! V3 payload wire format: header, parser, types, and errors.

/// Payload v3 error types.
pub mod errors;
/// Payload v3 header parsing and serialization.
pub mod header;
/// Payload v3 multi-version parser.
pub mod parser;
/// Payload v3 core types, constants, and flags.
pub mod types;
/// Payload v3 builder for constructing and serializing payloads.
pub mod writer;

pub use errors::PayloadV3ParseError;
pub use header::PayloadV3Header;
pub use parser::{parse_payload, ParsedPayload, V1Payload, V2Payload, V3Payload};
pub use types::{
    AuthAlgorithm, ExtensionEntry, PayloadFlags, ProtectionChannels, V3_CORE_SIZE,
    V3_DOMAIN_STRING, V3_MAGIC, V3_MAX_EMBEDDED_SIZE, V3_MAX_EXTENSION_COUNT,
    V3_MAX_EXTENSION_SIZE, V3_MAX_KEY_ID_LEN, V3_PAYLOAD_VERSION,
};
pub use writer::PayloadBuilder;
