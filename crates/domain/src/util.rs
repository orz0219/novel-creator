//! 共享小工具（哈希等）。放在 domain 以便 runtime / application / db 共用，
//! 避免各 crate 重复引 sha2。

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// 计算字符串的 sha256，返回小写 hex。
/// 用于 ReproducibilityMeta 的 prompt_hash 与检索文档内容 hash，
/// 保证跨进程 / 跨时间稳定（同一内容永远得到同一 hash）。
pub fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let out = hasher.finalize();
    let mut hex = String::with_capacity(out.len() * 2);
    for b in out {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

/// 由稳定 label 派生一个确定性 UUID（用于 RetrievedDocRef.id），
/// 使得同一检索文档在不同次生成中得到可复现的 id。
pub fn deterministic_uuid(label: &str) -> Uuid {
    let h = sha256_hex(label);
    let bytes = hex_to_16(&h);
    Uuid::from_slice(&bytes).unwrap_or_else(|_| Uuid::new_v4())
}

/// 把 64 字符 hex 的前 32 字符（=16 字节）解析为 [u8;16]。
fn hex_to_16(h: &str) -> [u8; 16] {
    let chars: Vec<u8> = h.bytes().collect();
    let mut out = [0u8; 16];
    for i in 0..16 {
        let hi = nib(chars.get(2 * i).copied());
        let lo = nib(chars.get(2 * i + 1).copied());
        out[i] = (hi << 4) | lo;
    }
    out
}

fn nib(b: Option<u8>) -> u8 {
    match b {
        Some(c) => match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => 0,
        },
        None => 0,
    }
}
