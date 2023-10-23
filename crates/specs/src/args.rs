pub fn parse_args(values: Vec<&str>) -> Vec<u64> {
    values
        .into_iter()
        .map(|v| {
            let [v, t] = v.split(":").collect::<Vec<&str>>()[..] else { todo!() };
            match t {
                "i64" => {
                    if v.starts_with("0x") {
                        vec![
                            u64::from_str_radix(String::from(v).trim_start_matches("0x"), 16)
                                .unwrap(),
                        ]
                    } else {
                        vec![v.parse::<u64>().unwrap()]
                    }
                }
                "bytes" => {
                    if !v.starts_with("0x") {
                        panic!("bytes input need start with 0x");
                    }
                    let bytes = hex::decode(String::from(v).trim_start_matches("0x")).unwrap();
                    bytes
                        .into_iter()
                        .map(|x| u64::from(x))
                        .collect::<Vec<u64>>()
                }
                "bytes-packed" => {
                    if !v.starts_with("0x") {
                        panic!("bytes input need start with 0x");
                    }
                    let bytes = hex::decode(String::from(v).trim_start_matches("0x")).unwrap();
                    let bytes = bytes.chunks(8);
                    bytes
                        .into_iter()
                        .map(|x| {
                            let mut data = [0u8; 8];
                            data[..x.len()].copy_from_slice(x);

                            u64::from_le_bytes(data)
                        })
                        .collect::<Vec<u64>>()
                }

                _ => {
                    panic!("Unsupported input data type: {}", t)
                }
            }
        })
        .flatten()
        .collect()
}
