use crate::error::{Error, Result};
use crate::types::{DmiValue, ProtectionContext, PLUS_DATA_MINING_PROPERTY};

impl super::RightsMetadataProtector {
    pub(super) fn jpeg_has_stego_metadata(&self, jpeg_data: &[u8]) -> bool {
        if jpeg_data.len() < 2 || jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 {
            return false;
        }
        let mut pos = 2;
        while pos + 2 <= jpeg_data.len() {
            if jpeg_data[pos] != 0xFF {
                pos += 1;
                continue;
            }
            let marker = jpeg_data[pos + 1];
            if marker == 0xD9 || marker == 0xDA {
                break;
            }
            if marker == 0x00 {
                pos += 1;
                continue;
            }
            if marker == 0xFE {
                if pos + 4 > jpeg_data.len() {
                    return false;
                }
                let comment_len =
                    u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
                if comment_len < 2 {
                    let Some(next) = pos.checked_add(2).and_then(|p| p.checked_add(comment_len))
                    else {
                        break;
                    };
                    pos = next;
                    continue;
                }
                let Some(comment_start) = pos.checked_add(4) else {
                    break;
                };
                let Some(raw_end) = comment_start.checked_add(comment_len.saturating_sub(2)) else {
                    break;
                };
                let comment_end = raw_end.min(jpeg_data.len());
                if comment_start > comment_end {
                    break;
                }
                let comment = &jpeg_data[comment_start..comment_end];
                if comment.starts_with(Self::STRUCTURED_COM_MAGIC) {
                    return true;
                }
                if let Ok(comment_str) = String::from_utf8(comment.to_vec()) {
                    if comment_str.starts_with("X-Protection-Seed: ")
                        || comment_str.starts_with("DMI-PROHIBITED: ")
                        || comment_str.starts_with("noai: ")
                        || comment_str.starts_with("Copyright: ")
                        || comment_str.starts_with("Contact: ")
                        || comment_str.starts_with("License: ")
                        || comment_str.starts_with("UsageTerms: ")
                        || comment_str.starts_with("DateCreated: ")
                        || comment_str.starts_with("AIConstraints: ")
                        || comment_str.starts_with("WebStatementOfRights: ")
                        || comment_str.starts_with("Creator: ")
                        || comment_str.starts_with("CreditLine: ")
                        || comment_str.starts_with("CopyrightOwner: ")
                        || comment_str.starts_with("LicensorName: ")
                        || comment_str.starts_with("LicensorEmail: ")
                        || comment_str.starts_with("LicensorURL: ")
                        || comment_str.starts_with("MetadataDate: ")
                        || comment_str.starts_with("NoticeAppliedAt: ")
                    {
                        return true;
                    }
                }
                pos = match pos.checked_add(2).and_then(|p| p.checked_add(comment_len)) {
                    Some(next) => next,
                    None => break,
                };
                continue;
            }
            if pos + 4 > jpeg_data.len() {
                return false;
            }
            let segment_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
            let Some(seg_end) = pos.checked_add(2).and_then(|p| p.checked_add(segment_len)) else {
                break;
            };
            if seg_end > jpeg_data.len() {
                break;
            }
            if marker == 0xED {
                let seg_data = &jpeg_data[pos + 4..seg_end];
                if Self::jpeg_payload_has_stego_properties(seg_data) {
                    return true;
                }
            }
            if marker == 0xE1 {
                let seg_data = &jpeg_data[pos + 4..seg_end];
                if seg_data.starts_with(b"http://ns.adobe.com/xap/1.0/\0") {
                    let xmp = &seg_data[29..];
                    if Self::xmp_has_stego_properties(xmp) {
                        return true;
                    }
                }
            }
            pos = seg_end;
        }
        false
    }

    pub(super) fn jpeg_payload_has_stego_properties(data: &[u8]) -> bool {
        if data.starts_with(b"Photoshop 3.0\0") {
            let mut scan = &data[14..];
            while scan.len() >= 6 {
                let resource_id = u16::from_be_bytes([scan[0], scan[1]]);
                if resource_id != 0x0404 {
                    break;
                }
                let pascal_len = scan[2] as usize;
                let Some(name_field_len) = pascal_len
                    .checked_add(1)
                    .and_then(|p| p.checked_add((pascal_len + 1) & 1))
                else {
                    break;
                };
                let Some(header_len) = 2usize.checked_add(name_field_len) else {
                    break;
                };
                if header_len.checked_add(4).is_none_or(|e| e > scan.len()) {
                    break;
                }
                let data_size = u32::from_be_bytes([
                    scan[header_len],
                    scan[header_len + 1],
                    scan[header_len + 2],
                    scan[header_len + 3],
                ]) as usize;
                let Some(data_start) = header_len.checked_add(4) else {
                    break;
                };
                let Some(data_end) = data_start.checked_add(data_size) else {
                    break;
                };
                if data_end > scan.len() {
                    break;
                }
                let iptc = &scan[data_start..data_end];
                if Self::iptc_has_stego_properties(iptc) {
                    return true;
                }
                let Some(aligned) = data_size.checked_add(data_size & 1) else {
                    break;
                };
                let Some(next) = header_len
                    .checked_add(4)
                    .and_then(|p| p.checked_add(aligned))
                else {
                    break;
                };
                if next >= scan.len() {
                    break;
                }
                scan = &scan[next..];
            }
            return false;
        }
        Self::iptc_has_stego_properties(data)
    }

    pub(super) fn iptc_has_stego_properties(data: &[u8]) -> bool {
        let mut pos = 0;
        while pos + 6 <= data.len() {
            if data[pos] != 0x1C {
                break;
            }
            let record = data[pos + 1];
            let tag = data[pos + 2];
            let val_len = u16::from_be_bytes([data[pos + 3], data[pos + 4]]) as usize;
            if record == 2 && matches!(tag, 0x05 | 0x78 | 0x7A | 0x7C | 0x7D) {
                return true;
            }
            let Some(next) = pos.checked_add(6).and_then(|p| p.checked_add(val_len)) else {
                break;
            };
            // Guard against non-advancing cursors on degenerate inputs.
            if next <= pos {
                break;
            }
            pos = next;
        }
        false
    }

    pub(super) fn xmp_has_stego_properties(xmp: &[u8]) -> bool {
        if xmp.windows(b"stegoeggo:".len()).any(|w| w == b"stegoeggo:") {
            return true;
        }
        if xmp
            .windows(PLUS_DATA_MINING_PROPERTY.len())
            .any(|w| w == PLUS_DATA_MINING_PROPERTY.as_bytes())
        {
            return true;
        }
        false
    }

    pub(super) fn inject_text_chunks_jpeg(
        &self,
        jpeg_data: &[u8],
        metadata: &[(Vec<u8>, Vec<u8>)],
        dmi: Option<DmiValue>,
        seed: Option<u64>,
        ctx: Option<&ProtectionContext>,
    ) -> Result<Vec<u8>> {
        if metadata.is_empty() && dmi.is_none() {
            return Ok(jpeg_data.to_vec());
        }

        if jpeg_data.len() < 2 || jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 {
            return Err(Error::Metadata("Invalid JPEG signature".to_string()));
        }

        let mut has_sos = false;
        {
            let mut p = 2;
            while p + 2 <= jpeg_data.len() {
                if jpeg_data[p] != 0xFF {
                    p += 1;
                    continue;
                }
                let m = jpeg_data[p + 1];
                if m == 0xDA || m == 0xD9 {
                    has_sos = m == 0xDA;
                    break;
                }
                if m == 0x00 {
                    p += 1;
                    continue;
                }
                if p + 4 > jpeg_data.len() {
                    break;
                }
                let sl = u16::from_be_bytes([jpeg_data[p + 2], jpeg_data[p + 3]]) as usize;
                let Some(se) = p.checked_add(2).and_then(|v| v.checked_add(sl)) else {
                    break;
                };
                if se > jpeg_data.len() {
                    break;
                }
                p = se;
            }
        }
        if !has_sos {
            return Err(Error::Metadata("JPEG missing SOS marker".to_string()));
        }

        let mut output = Vec::with_capacity(jpeg_data.len() + 500 * metadata.len() + 500);
        output.extend_from_slice(&jpeg_data[0..2]);

        let mut pos = 2;
        let mut inserted = false;

        while pos + 2 <= jpeg_data.len() {
            if jpeg_data[pos] != 0xFF {
                pos += 1;
                continue;
            }

            let marker = jpeg_data[pos + 1];

            if marker == 0xD9 {
                if !inserted {
                    self.inject_all_dmi_markers(&mut output, dmi, metadata, seed, ctx)?;
                    inserted = true;
                }
                output.extend_from_slice(&jpeg_data[pos..]);
                break;
            }

            if marker == 0xDA {
                if !inserted {
                    self.inject_all_dmi_markers(&mut output, dmi, metadata, seed, ctx)?;
                    inserted = true;
                }
                output.extend_from_slice(&jpeg_data[pos..]);
                break;
            }

            if marker == 0x00 {
                pos += 1;
                continue;
            }

            if pos + 4 > jpeg_data.len() {
                return Err(Error::ImageTruncated(format!(
                    "JPEG segment parsing truncated at byte {} (needed 4 bytes, had {})",
                    pos,
                    jpeg_data.len() - pos
                )));
            }

            let segment_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
            let segment_end = pos
                .checked_add(2)
                .and_then(|v| v.checked_add(segment_len))
                .ok_or_else(|| {
                    Error::Metadata(format!(
                        "JPEG segment length overflow at byte {} (segment_len={})",
                        pos, segment_len
                    ))
                })?;

            if segment_end > jpeg_data.len() {
                return Err(Error::ImageTruncated(format!(
                    "JPEG segment at byte {} claims length {} but only {} bytes remain",
                    pos,
                    segment_len,
                    jpeg_data.len().saturating_sub(pos + 2)
                )));
            }

            output.extend_from_slice(&jpeg_data[pos..segment_end]);
            pos = segment_end;
        }

        if !inserted {
            self.inject_all_dmi_markers(&mut output, dmi, metadata, seed, ctx)?;
        }

        Ok(output)
    }

    /// Injects all DMI markers: EXIF, IPTC-IIM, XMP, structured COM, and text comments.
    /// This ensures maximum compatibility across different image processing systems.
    pub(super) fn inject_all_dmi_markers(
        &self,
        output: &mut Vec<u8>,
        dmi: Option<DmiValue>,
        metadata: &[(Vec<u8>, Vec<u8>)],
        seed: Option<u64>,
        ctx: Option<&ProtectionContext>,
    ) -> Result<()> {
        let default_limits = crate::ResourceLimits::default();
        let limits = ctx.map(|c| c.resource_limits());
        let limits_ref = limits.as_ref().unwrap_or(&default_limits);

        let injected_field_count = metadata.len() + usize::from(dmi.is_some());
        limits_ref.check_metadata_field_count(injected_field_count)?;

        if let Some(dmi_val) = dmi {
            let exif_marker = Self::create_jpeg_exif_marker(&Self::generate_exif_dmi(dmi_val))?;
            output.extend_from_slice(&exif_marker);

            let iptc_marker =
                Self::create_jpeg_iptc_marker(&Self::generate_iptc_iim_dmi(dmi_val, seed))?;
            output.extend_from_slice(&iptc_marker);

            let xmp_bytes = Self::generate_xmp_dmi(dmi_val, seed);
            limits_ref.check_metadata_size("XMP", xmp_bytes.len(), limits_ref.max_xmp_bytes())?;
            let xmp_marker = Self::create_jpeg_xmp_marker(&xmp_bytes)?;
            output.extend_from_slice(&xmp_marker);
        }

        for (key, value) in metadata {
            let com_chunk = Self::create_jpeg_comment(key, value, Some(limits_ref))?;
            output.extend_from_slice(&com_chunk);
        }

        if let Some(context) = ctx {
            let structured_com = Self::generate_structured_com_marker(dmi, seed, context);
            output.extend_from_slice(&structured_com);
        }
        Ok(())
    }

    pub(super) fn create_jpeg_xmp_marker(xmp_data: &[u8]) -> Result<Vec<u8>> {
        let namespace = b"http://ns.adobe.com/xap/1.0/\0";
        let total_len = namespace.len() + xmp_data.len() + 2;
        let len_u16 = u16::try_from(total_len).map_err(|_| {
            Error::Metadata(format!(
                "JPEG XMP marker length {} exceeds u16::MAX (65535)",
                total_len
            ))
        })?;
        let mut marker = Vec::with_capacity(4 + total_len);
        marker.push(0xFF);
        marker.push(0xE1);
        marker.extend_from_slice(&len_u16.to_be_bytes());
        marker.extend_from_slice(namespace);
        marker.extend_from_slice(xmp_data);
        Ok(marker)
    }

    /// Creates an EXIF APP1 marker (0xFFE1) with DMI data in UserComment tag.
    /// Includes a valid minimal TIFF structure so EXIF parsers can locate the tag.
    ///
    /// Structure:
    /// - APP1 marker (0xFFE1) + length
    /// - "Exif\0\0" prefix
    /// - TIFF header: byte order "II" (little-endian) + magic 42 + IFD0 offset (8)
    /// - IFD0: 1 entry for UserComment (tag 0x9286)
    /// - UserComment data: ASCII charset identifier (8 bytes) + text
    pub(super) fn create_jpeg_exif_marker(exif_data: &[u8]) -> Result<Vec<u8>> {
        let tiff_prefix = b"Exif\x00\x00";

        // TIFF header: "II" (little-endian) + magic 42 + IFD0 offset
        let mut tiff_header = Vec::with_capacity(8);
        tiff_header.extend_from_slice(b"II"); // byte order
        tiff_header.extend_from_slice(&42u16.to_le_bytes()); // magic
        tiff_header.extend_from_slice(&8u32.to_le_bytes()); // IFD0 offset

        // IFD0: entry count (1) + one entry for UserComment (12 bytes)
        let mut ifd = Vec::with_capacity(14);
        ifd.extend_from_slice(&1u16.to_le_bytes()); // entry count
        ifd.extend_from_slice(&0x9286u16.to_le_bytes()); // tag: UserComment
        ifd.extend_from_slice(&2u16.to_le_bytes()); // type: ASCII (2)
        let count = u32::try_from(exif_data.len()).map_err(|_| {
            Error::Metadata(format!(
                "JPEG EXIF UserComment count {} exceeds u32::MAX",
                exif_data.len()
            ))
        })?;
        ifd.extend_from_slice(&count.to_le_bytes()); // count
                                                     // Value: if <= 4 bytes, stored inline (padded to 4);
                                                     // otherwise offset from TIFF start = 8 (header) + 2 (count) + 12 (entry) = 22.
        if exif_data.len() <= 4 {
            let mut inline = [0u8; 4];
            inline[..exif_data.len()].copy_from_slice(exif_data);
            ifd.extend_from_slice(&inline);
        } else {
            ifd.extend_from_slice(&22u32.to_le_bytes()); // offset to data
        }

        let total_len = tiff_prefix.len() + tiff_header.len() + ifd.len() + exif_data.len();
        let len_u16 = u16::try_from(total_len + 2).map_err(|_| {
            Error::Metadata(format!(
                "JPEG EXIF marker length {} exceeds u16::MAX (65535)",
                total_len + 2
            ))
        })?;
        let mut marker = Vec::with_capacity(4 + total_len);
        marker.push(0xFF);
        marker.push(0xE1);
        marker.extend_from_slice(&len_u16.to_be_bytes());
        marker.extend_from_slice(tiff_prefix);
        marker.extend_from_slice(&tiff_header);
        marker.extend_from_slice(&ifd);
        marker.extend_from_slice(exif_data);
        Ok(marker)
    }

    /// Creates an IPTC-IIM APP13 marker (0xFFED) containing DMI data.
    /// Wraps the IPTC data in a valid Photoshop resource envelope:
    /// "Photoshop 3.0\0" + Resource ID (0x0404) + Pascal string + data size + data.
    pub(super) fn create_jpeg_iptc_marker(iptc_data: &[u8]) -> Result<Vec<u8>> {
        let photoshop_id = b"Photoshop 3.0\0";

        // Photoshop resource envelope
        let mut resource = Vec::new();
        resource.extend_from_slice(&0x0404u16.to_be_bytes()); // Resource ID: IPTC-IIM
        resource.push(0x00); // Pascal string length (0 = empty)
        resource.push(0x00); // padding byte (even alignment)
        let data_size = u32::try_from(iptc_data.len()).map_err(|_| {
            Error::Metadata(format!(
                "JPEG IPTC data size {} exceeds u32::MAX",
                iptc_data.len()
            ))
        })?;
        resource.extend_from_slice(&data_size.to_be_bytes()); // data size
        resource.extend_from_slice(iptc_data); // IPTC-IIM data

        let mut data = Vec::new();
        data.extend_from_slice(photoshop_id);
        data.extend_from_slice(&resource);

        let len = u16::try_from(data.len() + 2).map_err(|_| {
            Error::Metadata(format!(
                "JPEG IPTC marker length {} exceeds u16::MAX (65535)",
                data.len() + 2
            ))
        })?;
        let mut marker = Vec::new();
        marker.push(0xFF);
        marker.push(0xED);
        marker.extend_from_slice(&len.to_be_bytes());
        marker.extend_from_slice(&data);
        Ok(marker)
    }

    #[allow(dead_code)]
    pub(super) fn extract_seed_from_jpeg(
        jpeg_data: &[u8],
        limits: Option<&crate::ResourceLimits>,
    ) -> Option<u64> {
        let mut pos = 2;
        let mut segment_count: usize = 0;
        while pos + 2 <= jpeg_data.len() {
            if jpeg_data[pos] != 0xFF {
                pos += 1;
                continue;
            }

            let marker = jpeg_data[pos + 1];

            if marker == 0xD9 || marker == 0xDA {
                break;
            }

            if marker == 0x00 {
                pos += 1;
                continue;
            }

            segment_count += 1;
            if let Some(lim) = limits {
                if segment_count > lim.max_jpeg_segments() {
                    return None;
                }
            }

            if marker == 0xFE {
                if pos + 4 > jpeg_data.len() {
                    return None;
                }
                let comment_len =
                    u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
                if comment_len < 2 {
                    let next = pos
                        .checked_add(2)
                        .and_then(|p| p.checked_add(comment_len))?;
                    pos = next;
                    continue;
                }
                let comment_start = pos.checked_add(4)?;
                let raw_end = comment_start.checked_add(comment_len.saturating_sub(2))?;
                let comment_end = raw_end.min(jpeg_data.len());
                if comment_start > comment_end {
                    return None;
                }
                let comment = &jpeg_data[comment_start..comment_end];

                if comment.starts_with(Self::STRUCTURED_COM_MAGIC) {
                    if let Some((seed, _level, _intensity)) =
                        Self::parse_structured_com_payload(comment)
                    {
                        return Some(seed);
                    }
                }

                if let Ok(comment_str) = String::from_utf8(comment.to_vec()) {
                    if let Some(seed_part) = comment_str.strip_prefix("X-Protection-Seed: ") {
                        return seed_part.trim().parse().ok();
                    }
                }
                let next = pos
                    .checked_add(2)
                    .and_then(|p| p.checked_add(comment_len))?;
                pos = next;
                continue;
            }

            if pos + 4 > jpeg_data.len() {
                return None;
            }
            let segment_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;

            if let Some(lim) = limits {
                if segment_len > lim.max_jpeg_segment_bytes() {
                    return None;
                }
            }

            if marker == 0xED && segment_len >= 2 {
                let seg_start = pos.checked_add(4)?;
                let raw_end = pos
                    .checked_add(2)
                    .and_then(|p| p.checked_add(segment_len))?;
                let seg_end = raw_end.min(jpeg_data.len());
                if seg_start > seg_end {
                    return None;
                }
                let seg_data = &jpeg_data[seg_start..seg_end];
                if let Some(seed) = Self::extract_seed_from_iptc(seg_data) {
                    return Some(seed);
                }
            }

            let next = pos
                .checked_add(2)
                .and_then(|p| p.checked_add(segment_len))?;
            pos = next;
        }
        None
    }

    pub(super) fn extract_seed_from_iptc(iptc_data: &[u8]) -> Option<u64> {
        let photoshop_header = b"Photoshop 3.0\x00";
        let search_start = iptc_data
            .windows(photoshop_header.len())
            .position(|w| w == photoshop_header)
            .map(|p| p + photoshop_header.len())
            .unwrap_or(0);

        let mut i = search_start;
        while i + 5 <= iptc_data.len() {
            if iptc_data[i] != 0x1C || iptc_data[i + 1] != 0x02 {
                i += 1;
                continue;
            }
            let tag = iptc_data[i + 2];
            let data_len = u16::from_be_bytes([iptc_data[i + 3], iptc_data[i + 4]]) as usize;
            let Some(data_start) = i.checked_add(5) else {
                break;
            };
            let Some(raw_end) = data_start.checked_add(data_len) else {
                break;
            };
            let data_end = raw_end.min(iptc_data.len());
            if data_start > data_end {
                break;
            }
            if tag == 0x05 {
                let data = &iptc_data[data_start..data_end];
                if let Ok(s) = std::str::from_utf8(data) {
                    if let Ok(seed) = s.trim().trim_end_matches('\0').parse::<u64>() {
                        return Some(seed);
                    }
                }
            }
            i = data_end;
            if !data_len.is_multiple_of(2) && i < iptc_data.len() {
                i += 1;
            }
        }
        None
    }

    pub(super) fn is_stego_owned_jpeg_segment(marker: u8, seg_data: &[u8]) -> bool {
        match marker {
            0xFE => {
                if seg_data.starts_with(Self::STRUCTURED_COM_MAGIC) {
                    return true;
                }
                if let Ok(s) = std::str::from_utf8(seg_data) {
                    s.starts_with("X-Protection-Seed: ")
                        || s.starts_with("DMI-PROHIBITED: ")
                        || s.starts_with("noai: ")
                        || s.starts_with("Copyright: ")
                        || s.starts_with("Contact: ")
                        || s.starts_with("License: ")
                        || s.starts_with("UsageTerms: ")
                        || s.starts_with("DateCreated: ")
                        || s.starts_with("AIConstraints: ")
                        || s.starts_with("WebStatementOfRights: ")
                        || s.starts_with("Creator: ")
                        || s.starts_with("CreditLine: ")
                        || s.starts_with("CopyrightOwner: ")
                        || s.starts_with("LicensorName: ")
                        || s.starts_with("LicensorEmail: ")
                        || s.starts_with("LicensorURL: ")
                        || s.starts_with("MetadataDate: ")
                        || s.starts_with("NoticeAppliedAt: ")
                } else {
                    false
                }
            }
            0xED => Self::jpeg_payload_has_stego_properties_static(seg_data),
            0xE1 if seg_data.starts_with(b"http://ns.adobe.com/xap/1.0/\0") => {
                Self::xmp_has_stego_properties(&seg_data[29..])
            }
            _ => false,
        }
    }

    pub(super) fn jpeg_payload_has_stego_properties_static(data: &[u8]) -> bool {
        if data.starts_with(b"Photoshop 3.0\0") {
            let mut scan = &data[14..];
            while scan.len() >= 6 {
                let resource_id = u16::from_be_bytes([scan[0], scan[1]]);
                if resource_id != 0x0404 {
                    break;
                }
                let pascal_len = scan[2] as usize;
                let Some(name_field_len) = pascal_len
                    .checked_add(1)
                    .and_then(|p| p.checked_add((pascal_len + 1) & 1))
                else {
                    break;
                };
                let Some(header_len) = 2usize.checked_add(name_field_len) else {
                    break;
                };
                if header_len.checked_add(4).is_none_or(|e| e > scan.len()) {
                    break;
                }
                let data_size = u32::from_be_bytes([
                    scan[header_len],
                    scan[header_len + 1],
                    scan[header_len + 2],
                    scan[header_len + 3],
                ]) as usize;
                let Some(data_start) = header_len.checked_add(4) else {
                    break;
                };
                let Some(data_end) = data_start.checked_add(data_size) else {
                    break;
                };
                if data_end > scan.len() {
                    break;
                }
                let iptc = &scan[data_start..data_end];
                if Self::iptc_has_stego_properties(iptc) {
                    return true;
                }
                let Some(aligned) = data_size.checked_add(data_size & 1) else {
                    break;
                };
                let Some(next) = header_len
                    .checked_add(4)
                    .and_then(|p| p.checked_add(aligned))
                else {
                    break;
                };
                if next >= scan.len() {
                    break;
                }
                scan = &scan[next..];
            }
            false
        } else {
            Self::iptc_has_stego_properties(data)
        }
    }

    pub(super) fn strip_stego_owned_jpeg(jpeg_data: &[u8]) -> Result<Vec<u8>> {
        if jpeg_data.len() < 2 || jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 {
            return Err(Error::Metadata("Invalid JPEG signature".to_string()));
        }
        let mut output = Vec::with_capacity(jpeg_data.len());
        output.extend_from_slice(&jpeg_data[0..2]);

        let mut pos = 2;
        while pos + 2 <= jpeg_data.len() {
            if jpeg_data[pos] != 0xFF {
                pos += 1;
                continue;
            }
            let marker = jpeg_data[pos + 1];
            if marker == 0xD9 || marker == 0xDA {
                output.extend_from_slice(&jpeg_data[pos..]);
                break;
            }
            if marker == 0x00 {
                pos += 1;
                continue;
            }
            if pos + 4 > jpeg_data.len() {
                return Err(Error::ImageTruncated(format!(
                    "JPEG segment parsing truncated at byte {}",
                    pos,
                )));
            }
            let segment_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
            if segment_len < 2 {
                return Err(Error::ImageTruncated(format!(
                    "JPEG segment at byte {} has invalid length {}",
                    pos, segment_len
                )));
            }
            let segment_end = pos
                .checked_add(2)
                .and_then(|v| v.checked_add(segment_len))
                .ok_or_else(|| {
                    Error::Metadata(format!("JPEG segment length overflow at byte {}", pos))
                })?;
            if segment_end > jpeg_data.len() {
                return Err(Error::ImageTruncated(format!(
                    "JPEG segment at byte {} claims length {} but only {} bytes remain",
                    pos,
                    segment_len,
                    jpeg_data.len().saturating_sub(pos + 2)
                )));
            }

            if marker == 0xFE || marker == 0xED || marker == 0xE1 {
                let seg_data = &jpeg_data[pos + 4..segment_end];
                if Self::is_stego_owned_jpeg_segment(marker, seg_data) {
                    pos = segment_end;
                    continue;
                }
            }

            output.extend_from_slice(&jpeg_data[pos..segment_end]);
            pos = segment_end;
        }

        Ok(output)
    }

    pub(super) fn collect_stego_owned_jpeg_keys(jpeg_data: &[u8]) -> Vec<Vec<u8>> {
        let mut keys = Vec::new();
        if jpeg_data.len() < 2 || jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 {
            return keys;
        }
        let mut pos = 2;
        while pos + 2 <= jpeg_data.len() {
            if jpeg_data[pos] != 0xFF {
                pos += 1;
                continue;
            }
            let marker = jpeg_data[pos + 1];
            if marker == 0xD9 || marker == 0xDA {
                break;
            }
            if marker == 0x00 {
                pos += 1;
                continue;
            }
            if pos + 4 > jpeg_data.len() {
                break;
            }
            let seg_len = u16::from_be_bytes([jpeg_data[pos + 2], jpeg_data[pos + 3]]) as usize;
            let Some(seg_end) = pos.checked_add(2).and_then(|p| p.checked_add(seg_len)) else {
                break;
            };
            if seg_end > jpeg_data.len() {
                break;
            }
            if marker == 0xFE {
                let seg_data = &jpeg_data[pos + 4..seg_end];
                if let Ok(s) = std::str::from_utf8(seg_data) {
                    for prefix in &[
                        "X-Protection-Seed: ",
                        "DMI-PROHIBITED: ",
                        "noai: ",
                        "Copyright: ",
                        "Contact: ",
                        "License: ",
                        "UsageTerms: ",
                        "DateCreated: ",
                        "AIConstraints: ",
                        "WebStatementOfRights: ",
                        "Creator: ",
                        "CreditLine: ",
                        "CopyrightOwner: ",
                        "LicensorName: ",
                        "LicensorEmail: ",
                        "LicensorURL: ",
                        "MetadataDate: ",
                        "NoticeAppliedAt: ",
                    ] {
                        if s.starts_with(prefix) {
                            let key = prefix.trim_end_matches(": ");
                            keys.push(key.as_bytes().to_vec());
                        }
                    }
                }
            }
            pos = seg_end;
        }
        keys
    }
}
