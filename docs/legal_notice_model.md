# Rights Notices and Technical Evidence

StegoEggo writes caller-supplied rights information into image metadata. An
optional hidden marker can provide a second, best-effort signal. Neither channel
enforces a restriction or guarantees that another system will preserve or honor
it.

See [format support](formats.md), [CLI usage](cli-usage.md), and the
[Rust API guide](rust-api.md) for implementation details.

## What the signals mean

| Signal | What it provides | What it does not provide |
|---|---|---|
| Rights metadata | A machine-readable policy and notice stored in the image | Proof that the claim is valid, or that a reader will honor it |
| Hidden marker | Optional redundant data embedded in image content | A durable watermark or proof that a model used the image |
| CRC32 | Detection of accidental payload corruption | Authentication or protection against deliberate changes |
| HMAC | Verification that a marker matches a shared secret key | Public identity, copyright ownership, or a trusted timestamp |

A valid HMAC means the marker data matches the supplied key. Anyone who knows
the key can create a valid marker. Keep keys outside image files and protect
them as credentials. A timestamp stored in metadata is a claim in the file, not
an independent timestamping service.

## Metadata by format

| Format | Rights metadata locations |
|---|---|
| PNG | Text chunks and XMP stored in an iTXt chunk |
| JPEG | COM markers and XMP |
| WebP | XMP |

The canonical data-mining policy is represented with the PLUS
`plus:DataMining` property. Other notice fields use format-appropriate
properties. Exact mappings and transformation behavior are described in
[format support](formats.md).

Metadata can be copied, edited, or removed. Hidden markers are also best effort:
re-encoding, resizing, and cropping may damage them. Tiled embedding can improve
recovery after some crops, but it does not guarantee recovery.

## Interpreting verification

`inspect` reports evidence found in a file. `verify` also provides a process
status suitable for scripts. Neither command determines whether a legal claim
is enforceable, who owns the image, when a notice was published, or whether an
AI system trained on it.

For stronger operational evidence, keep the original file and independent
records of what was distributed. Only make claims you are entitled to make.
This document is informational and is not legal advice.
