use std::error::Error;
use sha2::{Sha256, Digest};
use reqwest::get;
use serde::Deserialize;


// fn hash(data: &str) -> String {
//     let mut hasher = Sha256::new();
//     hasher.update(data);
//     let result = hasher.finalize();
//     result.iter().map(|byte| format!("{:02x}", byte)).collect()
// }

#[derive(Debug, Deserialize)]
struct LatestBlockData {
    hash: String,
    time: u64,
    height: u64,
}

#[derive(Debug, Deserialize)]
struct Block {
    bits: u64,
    ver: u64,
    prev_block: String,
    mrkl_root: String,
    time: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let latest_block = get("https://blockchain.info/latestblock").await?.json::<LatestBlockData>().await?;
    
    println!("Latest Block");
    println!("hash: {}", latest_block.hash);
    println!("time: {}", latest_block.time);
    println!("height: {}", latest_block.height);

    let block_data = get(format!("https://blockchain.info/rawblock/{}", latest_block.hash)).await?.json::<Block>().await?;

    println!("{:?}", block_data);

    let bits_in_hex = format!("{:x}", block_data.bits);

    let (exponent, coefficient) = bits_in_hex.split_at(2);
    let exponent = u64::from_str_radix(exponent, 16).unwrap();
    let bits = format!("{}{}", coefficient, "0".repeat((2*exponent-6).try_into().unwrap()));

    println!("bits: {}", bits); 
    
    
    let nonce = 0;
    let block_header = format!("{}{}{}{}{}", block_data.ver, block_data.prev_block, block_data.mrkl_root, block_data.bits, block_data.time);

    // let hash = hash(&hash("hello world"));
    Ok(())
}
