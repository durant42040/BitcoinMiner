use std::{error::Error, sync::Arc};
use sha2::{Digest, Sha256};
use serde::Deserialize;
use serde_json::from_str;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use tokio::{sync::Mutex, time::{interval, Duration}};
use hex;


#[derive(Debug, Deserialize)]
struct Block {
    blockIndex: u64,
    hash: String,
    time: u64,
    version: u64,
    bits: u64,
    nonce: u64,
    mrklRoot: String,
}

#[derive(Debug, Deserialize)]
struct BlockData {
    op: String,
    x: Option<Block>
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

    let (mut write, read) = ws_stream.split();
    write.send(Message::Text(r#"{"op":"blocks_sub"}"#.into())).await.unwrap();

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));
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
                        let data: BlockData = from_str(&text).unwrap();
                        if data.op == "block" {
                            println!("new block received");
                            println!("{:?}", data.x.unwrap());
                        } else {
                            println!("{:?}", data.op);
                        }
                    }
                    _ => {
                        println!("Received unknown message");
                    }
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
    println!("Block Header: {:?}", block_header);

    let block_hash = double_hash(block_header);
    println!("Block Hash: {:?}", block_hash);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    tokio::spawn(async {
        ws_connect().await.unwrap();
    }).await?;

    tokio::spawn(async {
        mine();
    }).await?;

    Ok(())
}

// { blockIndex: 849966, hash: "00000000000000000001c72bddcbdb95d42f8fb7be60b60dca6fb89f1f03c0ce", time: 1719669616, version: 1073676288, bits: 386096421, nonce: 1897988852, mrklRoot: "68a9aed40d36a400a0e3d8232b96f5204bc1c3ecec0afd529082f10e47a4a3b8" }
