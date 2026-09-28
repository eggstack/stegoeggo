use stegoeggo::{process_request_bytes, ProtectionRequest, RightsNotice, RightsPolicy};

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_hex(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut index = 0;
    while index < bytes.len() {
        out.push(hex_value(bytes[index])? << 4 | hex_value(bytes[index + 1])?);
        index += 2;
    }
    Some(out)
}

fn run(args: &[String]) -> Result<(), String> {
    if args.len() < 7 {
        return Err(format!(
            "usage: {} <input> <output> <metadata|marker> <seed-u64> <timestamp> <holder> [mac-hex]",
            args.first().map(String::as_str).unwrap_or("oracle")
        ));
    }
    let input = std::fs::read(&args[1]).map_err(|error| error.to_string())?;
    let seed: u64 = args[4].parse().map_err(|error| format!("seed: {error}"))?;
    let notice = RightsNotice::new().with_copyright_holder(args[6].clone());
    let policy = RightsPolicy::ProhibitedAiMlTraining;
    let mut request = match args[3].as_str() {
        "metadata" => ProtectionRequest::metadata_only(notice, policy),
        "marker" => ProtectionRequest::with_hidden_marker(notice, policy),
        other => return Err(format!("mode must be metadata or marker, got {other}")),
    }
    .with_seed(seed)
    .with_timestamp_override(args[5].clone());
    if let Some(mac_hex) = args.get(7) {
        let key = parse_hex(mac_hex).ok_or_else(|| "mac-hex must be even-length hex".to_string())?;
        request = request.with_mac_key(key);
    }
    let output = process_request_bytes(&input, &request).map_err(|error| error.to_string())?;
    std::fs::write(&args[2], output).map_err(|error| error.to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(error) = run(&args) {
        eprintln!("oracle: {error}");
        std::process::exit(1);
    }
}
