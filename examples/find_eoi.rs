fn main() {
    let data = std::fs::read("testimages/arttest.jpg").unwrap();

    // Find all EOIs
    let mut pos = 0;
    let mut count = 0;
    while pos < data.len() - 1 {
        if data[pos] == 0xFF && data[pos + 1] == 0xD9 {
            println!("EOI at position {}", pos);
            count += 1;
            if count > 5 {
                break;
            }
        }
        pos += 1;
    }

    // Check around first EOI
    println!("\nBytes around first EOI ({}):", 1010);
    #[allow(clippy::needless_range_loop)]
    for i in 1005..1020 {
        print!("{}:{:02X} ", i, data[i]);
    }
    println!();
}
