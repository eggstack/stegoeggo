use crate::output::config_err;
use std::fs;
use std::path::Path;

pub(crate) fn resolve_key_input(
    key_arg: &Option<String>,
    env_var: &str,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    if let Some(ref key_str) = key_arg {
        if key_str == "-" {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let hex_key = normalize_hex_key(&input);
            return Ok(Some(hex::decode(hex_key).map_err(|e| {
                config_err(format!("Invalid hex key from stdin: {}", e))
            })?));
        }
        if let Some(path_str) = key_str.strip_prefix('@') {
            let path = Path::new(path_str);
            if !path.exists() {
                return Err(config_err(format!("Key file not found: {}", path_str)));
            }
            let contents = fs::read_to_string(path).map_err(|e| {
                config_err(format!("Failed to read key file '{}': {}", path_str, e))
            })?;
            let hex_key = normalize_hex_key(&contents);
            return Ok(Some(hex::decode(&hex_key).map_err(|e| {
                config_err(format!("Invalid hex key in file: {}", e))
            })?));
        }
        let hex_key = normalize_hex_key(key_str);
        return Ok(Some(
            hex::decode(hex_key).map_err(|e| config_err(format!("Invalid hex key: {}", e)))?,
        ));
    }

    if !env_var.is_empty() {
        if let Ok(env_val) = std::env::var(env_var) {
            if !env_val.is_empty() {
                let hex_key = normalize_hex_key(&env_val);
                return Ok(Some(hex::decode(hex_key).map_err(|e| {
                    config_err(format!("Invalid hex key from {}: {}", env_var, e))
                })?));
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
