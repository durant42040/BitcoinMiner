pub fn to_little_endian(data: String) -> String {
    let bytes = hex::decode(data).expect("Decoding failed");
    let little_endian_bytes = bytes.into_iter().rev().collect::<Vec<_>>();
    hex::encode(little_endian_bytes)
}

pub fn get_target(bits : u64) -> String {
    let target = format!("{:x}", bits.clone());
    let (exponent, coefficient) = target.split_at(2);
    let exponent = u64::from_str_radix(exponent, 16).unwrap();
                    
    format!("{}{}{}", "0".repeat(18), coefficient, "0".repeat((2*exponent-6).try_into().unwrap()))
}

// pub fn rand_nonce() -> String {
//     let mut rng = rand::thread_rng();
//     let num: u32 = rng.gen();
//     format!("{:08x}", num)
// }