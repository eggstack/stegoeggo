fn main() {
    let data = std::fs::read("testimages/arttest.jpg").unwrap();

    // Find all SOF markers
    let mut pos = 0;
    let mut count = 0;
    while pos < data.len() - 1 && count < 10 {
        if data[pos] == 0xFF
            && (data[pos + 1] == 0xC0 || data[pos + 1] == 0xC1 || data[pos + 1] == 0xC2)
        {
            println!("SOF{} at {}", data[pos + 1] - 0xC0, pos);

            // Get dimensions
            if pos + 9 < data.len() {
                let h = ((data[pos + 5] as usize) << 8) | (data[pos + 6] as usize);
                let w = ((data[pos + 7] as usize) << 8) | (data[pos + 8] as usize);
                println!("  Size: {}x{}", h, w);
            }
            count += 1;
        }
        pos += 1;
    }
}
