use sha2::{Sha256, Digest};

pub fn compare_hash(hash : String, target: String) -> bool {
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


pub fn double_hash(data : String) -> String {
    hex::encode(Sha256::digest(&Sha256::digest(&hex::decode(data).unwrap())))
}