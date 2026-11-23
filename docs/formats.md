# Image Format Support

## Support at a glance

| Format | Rights metadata | Hidden marker |
|---|---|---|
| PNG | Supported | Pixel LSB, best effort |
| JPEG | Supported | DCT embedding on supported JPEG structures, best effort |
| WebP | Supported | Pixel LSB, best effort; output is encoded losslessly |

The CLI detects the input format and keeps it unless `--format` requests a
conversion. Marker carrier selection follows the output format: JPEG uses DCT;
PNG and WebP use pixel-domain LSB.

The main `stegoeggo` byte-processing path supports rights metadata for WebP.
When a hidden marker is requested, it processes pixels and writes a losslessly
encoded WebP. Later lossy re-encoding can destroy the marker. The optional
`stegoeggo-stego` WebP carrier is a separate API: it accepts still lossless
VP8L input and rejects lossy or animated WebP. The root pipeline does not route
to that optional carrier implicitly.

## What is written

Rights metadata is the primary signal. When a policy is specified, the library
writes it as the PLUS `plus:DataMining` property. Supplied notice fields use
format-specific metadata: PNG text and XMP, JPEG COM and XMP, or WebP XMP.

When the format-specific byte update path supports it, unrelated container
metadata is preserved. Raster processing, format conversion, or a later
re-encode may remove metadata unrelated to the new rights notice. Same-format
JPEG marker processing uses a byte path that avoids pixel re-encoding.

Hidden markers are optional and best effort:

- PNG and WebP output use pixel-domain LSB embedding.
- JPEG output uses DCT-domain embedding. Some unsupported JPEG structures may
  receive only a seed hint rather than a full marker.
- Tiled mode repeats the payload in image tiles. It can improve recovery after
  some crops, but it does not guarantee recovery.

## Transformations

| Operation | Rights metadata | Hidden marker |
|---|---|---|
| Byte-for-byte copy | Preserved | Preserved |
| Social-media upload or optimization | May be stripped or changed | May be removed or damaged |
| Screenshot | Usually lost | Lost |
| Resize or crop | Depends on the tool | Often damaged; tiled mode may help with some crops |
| Metadata stripping | Removed | May remain if pixel data is unchanged |
| Lossy re-encoding | Depends on the tool | Usually damaged or removed |

Do not assume another service preserves either channel. Test the actual delivery
path if preservation matters.

## Standards and compatibility

Current policy output uses the PLUS License Data Format `plus:DataMining`
property. Older policy metadata and legacy payload versions remain readable
where supported. Current hidden payloads use version 3.

StegoEggo does not claim C2PA compatibility or robust forensic watermarking.
Those are separate trust and provenance systems.
