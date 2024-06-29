use std::{error::Error, sync::Arc};
use sha2::{Digest, Sha256};
use serde::Deserialize;
use serde_json::from_str;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::time::{interval, Duration};
use futures_util::{StreamExt, SinkExt};
use tokio::sync::Mutex;
use hex;

struct Block {
    version: String,
    prev_block: String,
    merkle_root: String,
    time: String,
    bits: String,
    nonce: String,
}

#[derive(Debug, Deserialize)]
struct BlockData {
    blockIndex: u32,
    hash: String,
    time: u32,
    version: u32,
    bits: u32,
    nonce: u32,
    mrklRoot: String,
}

#[derive(Debug, Deserialize)]
struct WSData {
    op: String,
    x: Option<BlockData>
}


fn double_hash(data : String) -> String {
    hex::encode(Sha256::digest(&Sha256::digest(&hex::decode(data).unwrap())))
}

fn to_little_endian(data: &str) -> String {
    let bytes = hex::decode(data).expect("Decoding failed");
    let little_endian_bytes = bytes.into_iter().rev().collect::<Vec<_>>();
    hex::encode(little_endian_bytes)
}

async fn ws_connect() -> Result<(), Box<dyn Error>> {
    let (ws_stream, _) = connect_async("wss://ws.blockchain.info/inv")
        .await?;

    println!("Connected to WebSocket");

    // subscribe to new blocks
    let (mut write, read) = ws_stream.split();
    write.send(Message::Text(r#"{"op":"blocks_sub"}"#.into())).await.unwrap();


    // ping every 60 seconds to keep connection alive
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            write.send(Message::Text(r#"{"op":"ping"}"#.into())).await.unwrap();
        }
    });

    read.for_each(|message| async {
        match message {
            Ok(msg) => {
                match msg {
                    Message::Text(text) => {
                        let data: WSData = from_str(&text).unwrap();
                        if data.op == "block" {
                            println!("New block received");
                            println!("{:?}", data.x.unwrap());
                        } else {
                            println!("{:?}", data.op);
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
    }).await;

    Ok(())
}

fn mine() {
    let version = "01000000";
    let prev_block =  "0000000000000000000000000000000000000000000000000000000000000000";
    let merkle_root =  "3BA3EDFD7A7B12B27AC72C3E67768F617FC81BC3888A51323A9FB8AA4B1E5E4A";
    let time =  "29AB5F49";
    let bits =  "FFFF001D";
    let nonce =  "1DAC2B7C";


    let block_header = format!("{}{}{}{}{}{}", version, prev_block, merkle_root, time, bits, nonce);
    // println!("Block Header: {:?}", block_header);

    let block_hash = double_hash(block_header);
    // println!("Block Hash: {:?}", block_hash);
}

fn increment(x: &mut i32) {
    *x += 1;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    let block = Arc::new(Mutex::new(Block {
        version: "".to_string(),
        prev_block: "".to_string(),
        merkle_root: "".to_string(),
        time: "".to_string(),
        bits: "".to_string(),
        nonce: "".to_string(),
    }));
     
    let block_clone = Arc::clone(&block);

    tokio::spawn(async move {
        let block = &*block_clone.lock().await;
        ws_connect().await.unwrap();
    });

    tokio::spawn(async move {
        loop {
            mine();
        }
    }).await.unwrap();
    
    Ok(())
}

// let x = Arc::new(Mutex::new(0));

//     let x_clone = Arc::clone(&x);
//     tokio::spawn(async move {
//         // let block = &*block_clone.lock().await;
//         // ws_connect().await.unwrap();
//         let x = &mut *x_clone.lock().await;
//         increment(x);
//         println!("x: {:?}", x);
//     });

//     let x_clone = Arc::clone(&x);
//     tokio::spawn(async move {
//         let x = *x_clone.lock().await;
//         loop {
//             // mine();
//             println!("mine x: {:?}", x);
//         }
//     }).await.unwrap();