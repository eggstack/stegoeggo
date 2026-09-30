use crate::output::config_err;
use std::fs;
use std::path::Path;

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
            use std::io::Read as _;
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
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
            let contents = fs::read_to_string(path)
                .map_err(|e| config_err(format!("Failed to read key file '{path_str}': {e}")))?;
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
