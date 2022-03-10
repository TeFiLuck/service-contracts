use sha2::Digest;

pub fn calculate_sha256(input: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(input);
    format!("{:X}", hasher.finalize()).to_lowercase()
}
