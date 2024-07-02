use std::sync::Arc;
use sha2::{Digest, Sha256};
use serde::Deserialize;
use serde_json::from_str;
use tokio_tungstenite::{connect_async, tungstenite::{protocol::Message, error::Error}};
use tokio::time::{interval, Duration};
use futures_util::{StreamExt, SinkExt};
use tokio::sync::Mutex;
// use rand::Rng;
use hex;


#[derive(Debug, Deserialize)]
struct Block {
    hash: String,
    prev_block: String,
    time: String,
    version: String,
    bits: String,
    #[serde(rename = "mrklRoot")]
    merkle_root: String,
    target: String,
}

#[derive(Debug, Deserialize)]
struct BlockData {
    hash: String,
    time: u64,
    version: u64,
    bits: u64,
    #[serde(rename = "mrklRoot")]
    merkle_root: String,
}


#[derive(Debug, Deserialize)]
struct BlockResponse {
    op: String,
    x: Option<BlockData>
}

fn compare_hash(hash : String, target: String) -> bool {
    let hash = hex::decode(hash).unwrap();
    let target = hex::decode(target).unwrap();

    for i in 0..32 {
        if hash[i] > target[i] {
            return false;
        } else if hash[i] < target[i] {
            return true;
        }
    }
    return true;
}


fn double_hash(data : String) -> String {
    hex::encode(Sha256::digest(&Sha256::digest(&hex::decode(data).unwrap())))
}

fn to_little_endian(data: String) -> String {
    let bytes = hex::decode(data).expect("Decoding failed");
    let little_endian_bytes = bytes.into_iter().rev().collect::<Vec<_>>();
    hex::encode(little_endian_bytes)
}

// fn rand_nonce() -> String {
//     let mut rng = rand::thread_rng();
//     let num: u32 = rng.gen();
//     format!("{:08x}", num)
// }

fn mine(block : &Block, nonce: u32) {
    let nonce = format!("{:08x}", nonce);

    let block_header = format!("{}{}{}{}{}{}", block.version, block.prev_block, block.merkle_root, block.time, block.bits, nonce);
    let block_hash = double_hash(block_header);

    println!("hash: {}, nonce: {}", block_hash, nonce);

    if compare_hash(to_little_endian(block_hash.clone()), block.target.clone()) {
        println!("Block mined!");
        println!("Block hash: {}", block_hash);
        println!("Nonce: {}", nonce);
    } 
}

fn get_target(bits : u64) -> String {
    let target = format!("{:x}", bits.clone());
    let (exponent, coefficient) = target.split_at(2);
    let exponent = u64::from_str_radix(exponent, 16).unwrap();
                    
    format!("{}{}{}", "0".repeat(18), coefficient, "0".repeat((2*exponent-6).try_into().unwrap()))
}

async fn handle_message(message: Result<Message, Error>, block_clone: Arc<Mutex<Block>>) {
    match message {
        Ok(msg) => {
            match msg {
                Message::Text(text) => {
                    let data: BlockResponse = from_str(&text).unwrap();
                    if data.op == "block" {
                        let new_block = data.x.unwrap();

                        println!("New block received");
                        println!("{:?}", new_block);

                        let target = get_target(new_block.bits);

                        let mut block = block_clone.lock().await;
                        *block = Block {
                            version: to_little_endian(format!("{:x}", new_block.version)),
                            prev_block: block.hash.clone(),
                            hash: to_little_endian(new_block.hash),
                            merkle_root: to_little_endian(new_block.merkle_root),
                            time: to_little_endian(format!("{:x}", new_block.time)),
                            bits: to_little_endian(format!("{:x}", new_block.bits.clone())),
                            target: target,
                        };
                    } 
                }
                Message::Close(_) => {
                    println!("Connection closed");
                }
                _ => {}
            }
        }
        Err(e) => {
            println!("Error receiving message: {:?}", e);
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let latest_hash = reqwest::get("https://blockchain.info/q/latesthash").await?.text().await?;
    
    let block = Arc::new(Mutex::new(Block {
        hash: to_little_endian(latest_hash),
        prev_block: "".to_string(),
        time: "".to_string(),
        version: "".to_string(),
        bits: "".to_string(),
        merkle_root: "".to_string(),
        target: "".to_string(),
    }));


    let (ws_stream, _) = connect_async("wss://ws.blockchain.info/inv")
        .await?;

    println!("Connected to WebSocket");

    // subscribe to new blocks
    let (mut write, read) = ws_stream.split();
    write.send(Message::Text(r#"{"op":"blocks_sub"}"#.into())).await.unwrap();


    // ping every 60 seconds to keep connection alive
    let ping = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            write.send(Message::Text(r#"{"op":"ping"}"#.into())).await.unwrap_or_else(
                |e| println!("Error: {:?}", e)
            );
        }
    });


    let block_clone = Arc::clone(&block);
    let block_listener = tokio::spawn(async move {
        read.for_each(|message| async {
            let block_clone = Arc::clone(&block_clone);
            handle_message(message, block_clone).await;
        }).await;
    });

    let block_clone = Arc::clone(&block);
    let block_miner = tokio::spawn(async move {
        loop {
            let block = block_clone.lock().await;
            if block.version != "" {
                println!("Begin mining...");
                break;
            }
        }

        let mut nonce = 0;
        loop {
            // nonce = rand_nonce();
            let block = block_clone.lock().await;
            mine(&block, nonce);
            nonce += 1;
        }
    });

    tokio::try_join!(ping, block_listener, block_miner)?;

    Ok(())
}

// to_little_endian: 18.125µs move to front
// nonce generator: 1.334µs 
// target: 19.291µs move to front
// formatting: 2.376µs
// hashing: 52.541µs
// compare hash: 26.293µs
  

// Block { 
// hash: "9082cb87da2cba58e2cf54384d412d0d8608be3eff5100000000000000000000", 
// prev_block: "000000000000000000022208fcd649361fefa37c874cc9bdba471b1f73025c23", 
// time: "c8b08366", 
// version: "0080b527", 
// bits: "0000000000000000005d031700000000000000000000000000000000000000000000000000000000000000000000", 
// merkle_root: "ab1c786b0b023ad1cd9fc3ada5640ac188cedd2c4d7de10f6caf565613a2e145" }