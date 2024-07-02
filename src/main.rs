pub mod hash;
pub mod utils;

use hash::hash::{compare_hash, double_hash};
use utils::utils::{to_little_endian, get_target};

use std::sync::Arc;
use serde::Deserialize;
use serde_json::from_str;
use tokio_tungstenite::{connect_async, tungstenite::{protocol::Message, error::Error}};
use tokio::time::{interval, Duration};
use futures_util::{StreamExt, SinkExt};
use tokio::sync::Mutex;
// use rand::Rng;


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


async fn handle_message(message: Result<Message, Error>, block_clone: Arc<Mutex<Block>>) {
    let msg = message.unwrap_or_else(
        |e| {
            println!("Error: {:?}", e);
            Message::Close(None)
        }
    );
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
