use std::sync::Arc;
use sha2::{Digest, Sha256};
use serde::Deserialize;
use serde_json::from_str;
use tokio_tungstenite::{connect_async, tungstenite::{protocol::Message, error::Error}};
use tokio::time::{interval, Duration};
use futures_util::{StreamExt, SinkExt};
use tokio::sync::Mutex;
use hex;


#[derive(Debug, Deserialize)]
struct Block {
    hash: Option<String>,
    prev_block: Option<String>,
    time: Option<u32>,
    version: Option<u32>,
    bits: Option<u32>,
    nonce: Option<u32>,
    #[serde(rename = "mrklRoot")]
    merkle_root: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BlockData {
    op: String,
    x: Option<Block>
}

fn compare_hash(hash : String, target: String) -> bool {
    println!("hash: {:?}", hash);
    println!("target: {:?}", target);

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


fn mine(block : &Block) {
    let version = to_little_endian(format!("{:x}", block.version.unwrap()));
    let prev_block =  to_little_endian(block.prev_block.clone().unwrap());
    let merkle_root =  to_little_endian(block.merkle_root.clone().unwrap());
    let time =  to_little_endian(format!("{:x}", block.time.unwrap()));
    let bits =  to_little_endian(format!("{:x}", block.bits.unwrap()));
    let nonce =  to_little_endian(format!("{:x}", 3226147965u32));

    let target = format!("{:x}", block.bits.unwrap());
    let (exponent, coefficient) = target.split_at(2);
    let exponent = u64::from_str_radix(exponent, 16).unwrap();

    let target = format!("{}{}{}", "0".repeat(18), coefficient, "0".repeat((2*exponent-6).try_into().unwrap()));



    let block_header = format!("{}{}{}{}{}{}", version, prev_block, merkle_root, time, bits, nonce);

    let block_hash = double_hash(block_header);

    if compare_hash(to_little_endian(block_hash.clone()), target) {
        println!("Block mined!");
        println!("Block hash: {:?}", block_hash);
        println!("Nonce: {:?}", nonce);
    } 
}

async fn handle_message(message: Result<Message, Error>, block_clone: Arc<Mutex<Block>>) {
    match message {
        Ok(msg) => {
            match msg {
                Message::Text(text) => {
                    let data: BlockData = from_str(&text).unwrap();
                    if data.op == "block" {
                        println!("New block received");
                        let new_block = data.x.unwrap();
                        println!("new_block: {:?}", new_block);
                        let mut block = block_clone.lock().await;
                        *block = Block {
                            version: new_block.version,
                            prev_block: block.hash.clone(),
                            hash: new_block.hash,
                            merkle_root: new_block.merkle_root,
                            time: new_block.time,
                            bits: new_block.bits,
                            nonce: new_block.nonce,
                        };
                    } else {
                        println!("{}", data.op);
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
    let block = Arc::new(Mutex::new(Block {
        hash: None,
        prev_block: None,
        time: None,
        version: None,
        bits: None,
        nonce: None,
        merkle_root: None,
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
            write.send(Message::Text(r#"{"op":"ping"}"#.into())).await.unwrap();
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
            if block.version.is_some() {
                println!("Begin mining...");
                break;
            }
        }
        loop {
            let block = block_clone.lock().await;
            mine(&block);
        }
    });

    tokio::try_join!(ping, block_listener, block_miner)?;

    Ok(())
}

// BlockData { blockIndex: 850080, 
// hash: "000000000000000000011c34ec3b3f9cabd4254ef45882c12ed929e3570b5b50", 
// time: 1719734992, 
// version: 536911872, 
// bits: 386096421, 
// nonce: 1137605160, 
// mrklRoot: "0443824439444b0d9d6ca03cc4ef3bdb1c7ed123f7a1cfc5d3c035d1d7e92e91" }