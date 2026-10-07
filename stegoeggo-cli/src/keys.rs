use crate::output::config_err;
use std::fs;
use std::io::Read;
use std::path::Path;

/// Upper bound on key material accepted from stdin or a key file.
///
/// A MAC key is a short hex token, so this is far above any legitimate value
/// while keeping an external read from growing the heap without limit. Oversized
/// input is rejected rather than truncated, because a truncated read would decode
/// into a *different* key instead of failing.
const MAX_KEY_INPUT_BYTES: u64 = 4096;

fn read_key_text(reader: impl Read, source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    reader
        .take(MAX_KEY_INPUT_BYTES + 1)
        .read_to_end(&mut buf)
        .map_err(|e| config_err(format!("Failed to read key from {source}: {e}")))?;
    if buf.len() as u64 > MAX_KEY_INPUT_BYTES {
        return Err(config_err(format!(
            "Key from {source} exceeds the {MAX_KEY_INPUT_BYTES}-byte limit"
        )));
    }
    String::from_utf8(buf).map_err(|_| config_err(format!("Key from {source} is not valid UTF-8")))
}

pub(crate) fn resolve_key_input(
    key_arg: &Option<String>,
    env_var: &str,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    fn reject_empty(key: Vec<u8>, source: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if key.is_empty() {
            return Err(config_err(format!(
                "Empty key from {source}: provide non-empty hex"
            )));
        }
        Ok(key)
    }
    if let Some(ref key_str) = key_arg {
        if key_str == "-" {
            let input = read_key_text(std::io::stdin().lock(), "stdin")?;
            let hex_key = normalize_hex_key(&input);
            let decoded = hex::decode(hex_key)
                .map_err(|e| config_err(format!("Invalid hex key from stdin: {e}")))?;
            return Ok(Some(reject_empty(decoded, "stdin")?));
        }
        if let Some(path_str) = key_str.strip_prefix('@') {
            let path = Path::new(path_str);
            if !path.exists() {
                return Err(config_err(format!("Key file not found: {path_str}")));
            }
            let file = fs::File::open(path)
                .map_err(|e| config_err(format!("Failed to read key file '{path_str}': {e}")))?;
            let contents = read_key_text(file, &format!("file '{path_str}'"))?;
            let hex_key = normalize_hex_key(&contents);
            let decoded = hex::decode(&hex_key)
                .map_err(|e| config_err(format!("Invalid hex key in file: {e}")))?;
            return Ok(Some(reject_empty(decoded, "key file")?));
        }
        let hex_key = normalize_hex_key(key_str);
        let decoded =
            hex::decode(hex_key).map_err(|e| config_err(format!("Invalid hex key: {e}")))?;
        return Ok(Some(reject_empty(decoded, "key argument")?));
    }

    if !env_var.is_empty() {
        if let Ok(env_val) = std::env::var(env_var) {
            if !env_val.is_empty() {
                let hex_key = normalize_hex_key(&env_val);
                let decoded = hex::decode(hex_key)
                    .map_err(|e| config_err(format!("Invalid hex key from {env_var}: {e}")))?;
                return Ok(Some(reject_empty(decoded, env_var)?));
            }
        }
    }

    Ok(None)
}

pub(crate) fn normalize_hex_key(value: &str) -> String {
    value.chars().filter(|c| !c.is_whitespace()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn hex_key_normalization_removes_all_whitespace() {
        assert_eq!(normalize_hex_key(" ab\tcd\nef\r"), "abcdef");
    }

    #[test]
    fn key_file_accepts_inner_whitespace() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("key.txt");
        fs::write(&path, "ab cd\tef\n").unwrap();
        let key_arg = Some(format!("@{}", path.display()));

        assert_eq!(
            resolve_key_input(&key_arg, "").unwrap(),
            Some(vec![0xab, 0xcd, 0xef])
        );
    }
}
