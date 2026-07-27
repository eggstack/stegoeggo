#!/usr/bin/env bash
#
# Independent conformance fixture generator.
#
# These fixtures are authored using Python (stdlib only) and ImageMagick,
# NOT using StegoEggo production metadata writers. They satisfy the
# "genuinely independent" requirement of Plan 028 Workstream E.
#
# Base image: 64x64 solid red PNG created with ImageMagick `convert`.
# Metadata is written as raw XMP (in iTXt chunks) and standard PNG tEXt
# chunks using Python, with canonical namespace URIs
# (plus: http://ns.useplus.org/ldf/xmp/1.0/).
#
# Authoring tools:
#   ImageMagick convert (version recorded at generation time)
#   Python 3 (stdlib only, version recorded at generation time)
#
# License: MIT (same as stegoeggo)
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

PYTHON_VERSION=$(python3 --version 2>&1)
IMAGEMAGICK_VERSION=$(convert --version 2>&1 | head -1)
GENERATOR_SHA=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
GENERATION_COMMAND="tests/fixtures/conformance/independent/generate_independent_fixtures.sh"

echo "Generating independent fixtures..."
echo "  Python: $PYTHON_VERSION"
echo "  ImageMagick: $IMAGEMAGICK_VERSION"
echo "  Generator SHA: $GENERATOR_SHA"

python3 << 'PYEOF'
import struct, zlib, os, subprocess

SCRIPT_DIR = os.getcwd()

def write_png_chunk(chunk_type, chunk_data):
    crc = zlib.crc32(chunk_type + chunk_data) & 0xffffffff
    return struct.pack('>I', len(chunk_data)) + chunk_type + chunk_data + struct.pack('>I', crc)

def write_png_with_xmp_and_text(base_png, output_png, xmp_str, text_fields):
    with open(base_png, 'rb') as f:
        data = f.read()
    new_chunks = b''
    for key, value in text_fields:
        text_data = key.encode('utf-8') + b'\x00' + value.encode('utf-8')
        new_chunks += write_png_chunk(b'tEXt', text_data)
    xmp_bytes = xmp_str.encode('utf-8')
    keyword = b'XML:com.adobe.xmp\x00'
    itxt_data = keyword + b'\x00\x00\x00\x00' + xmp_bytes
    new_chunks += write_png_chunk(b'iTXt', itxt_data)
    iend_pos = data.find(b'IEND')
    iend_start = iend_pos - 4
    new_data = data[:iend_start] + new_chunks + data[iend_start:]
    with open(output_png, 'wb') as f:
        f.write(new_data)

subprocess.run(['convert', '-size', '64x64', 'xc:red', 'base.png'], check=True)
base_png = os.path.join(SCRIPT_DIR, 'base.png')

PLUS_NS = 'http://ns.useplus.org/ldf/xmp/1.0/'

xmp_legacy = f"""<?xpacket begin='' id='W5M0MpCehiHzreSzNTczkc9d'?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="python-stdlib-png-xmp-injector">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:plus="{PLUS_NS}">
   <dc:rights>
    <rdf:Alt><rdf:li xml:lang="x-default">Copyright (c) 2024 Independent Test</rdf:li></rdf:Alt>
   </dc:rights>
   <dc:creator>
    <rdf:Seq><rdf:li>Independent Creator</rdf:li></rdf:Seq>
   </dc:creator>
   <plus:DataMining>DMI-PROHIBITED-AIMLTRAINING</plus:DataMining>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end='w'?>"""
write_png_with_xmp_and_text(base_png, os.path.join(SCRIPT_DIR, 'legacy_dmi_prohibited.png'), xmp_legacy, [
    ('Copyright', 'Copyright (c) 2024 Independent Test'),
    ('Creator', 'Independent Creator'),
])

xmp_conflict = f"""<?xpacket begin='' id='W5M0MpCehiHzreSzNTczkc9d'?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="python-stdlib-png-xmp-injector">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:plus="{PLUS_NS}"
    xmlns:Iptc4xmpExt="http://iptc.org/std/Iptc4xmpExt/2008-02-29/"
    plus:DataMining="DMI-PROHIBITED-AIMLTRAINING"
    Iptc4xmpExt:DMI="Allowed">
   <dc:rights>
    <rdf:Alt><rdf:li xml:lang="x-default">Copyright (c) 2024 Conflict Test</rdf:li></rdf:Alt>
   </dc:rights>
   <dc:creator>
    <rdf:Seq><rdf:li>Independent Creator</rdf:li></rdf:Seq>
   </dc:creator>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end='w'?>"""
write_png_with_xmp_and_text(base_png, os.path.join(SCRIPT_DIR, 'conflict_canonical_legacy.png'), xmp_conflict, [
    ('Copyright', 'Copyright (c) 2024 Conflict Test'),
    ('Creator', 'Independent Creator'),
    ('DMI-PROHIBITED', 'ProhibitedAiMlTraining'),
])

xmp_preservation = f"""<?xpacket begin='' id='W5M0MpCehiHzreSzNTczkc9d'?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="python-stdlib-png-xmp-injector">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:plus="{PLUS_NS}">
   <dc:rights>
    <rdf:Alt><rdf:li xml:lang="x-default">https://independent.test/rights</rdf:li></rdf:Alt>
   </dc:rights>
   <dc:creator>
    <rdf:Seq><rdf:li>Independent Preservation Creator</rdf:li></rdf:Seq>
   </dc:creator>
   <xmp:WebStatement>https://independent.test/rights</xmp:WebStatement>
   <plus:DataMining>DMI-PROHIBITED-AIMLTRAINING</plus:DataMining>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end='w'?>"""
write_png_with_xmp_and_text(base_png, os.path.join(SCRIPT_DIR, 'preservation_custom_xmp.png'), xmp_preservation, [
    ('Copyright', 'https://independent.test/rights'),
    ('Creator', 'Independent Preservation Creator'),
])

os.remove(base_png)
print("Independent fixtures generated:")
print("  legacy_dmi_prohibited.png")
print("  conflict_canonical_legacy.png")
print("  preservation_custom_xmp.png")
PYEOF

echo ""
echo "Regenerate digests with:"
echo "  sha256sum legacy_dmi_prohibited.png conflict_canonical_legacy.png preservation_custom_xmp.png"
