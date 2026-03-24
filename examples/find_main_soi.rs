fn main() {
    let data = std::fs::read("testimages/arttest.jpg").unwrap();

    // Find all SOIs and their positions
    let mut pos = 0;
    let mut soi_list = Vec::new();
    while pos < data.len() - 1 {
        if data[pos] == 0xFF && data[pos + 1] == 0xD8 {
            soi_list.push(pos);
        }
        pos += 1;
    }

    println!("SOIs found at: {:?}", soi_list);

    // Now find all SOF positions
    let mut pos = 0;
    let mut sof_list = Vec::new();
    while pos < data.len() - 10 {
        if data[pos] == 0xFF
            && (data[pos + 1] == 0xC0 || data[pos + 1] == 0xC1 || data[pos + 1] == 0xC2)
        {
            let h = ((data[pos + 5] as usize) << 8) | (data[pos + 6] as usize);
            let w = ((data[pos + 7] as usize) << 8) | (data[pos + 8] as usize);
            sof_list.push((pos, w, h));
        }
        pos += 1;
    }

    println!("\nSOFs found:");
    for (p, w, h) in &sof_list {
        println!("  pos={}, size={}x{}", p, w, h);
    }

    // For each SOF, find the nearest preceding SOI
    println!("\nSOI for each SOF:");
    for (sof_pos, w, h) in &sof_list {
        let mut best_soi = 0;
        for &soi in &soi_list {
            if *sof_pos > soi {
                best_soi = soi;
            }
        }
        println!("  SOF at {} ({}x{}) -> SOI at {}", sof_pos, w, h, best_soi);
    }
}
