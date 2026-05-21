use crate::config::Config;
use crate::network::send_tcp_message;
use crate::protocol::Message;

use base64::{engine::general_purpose, Engine as _};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub async fn send_file(cfg: Config, path: PathBuf) {
    if !cfg.drops_enabled {
        return;
    }

    if !path.exists() || !path.is_file() {
        return;
    }

    let name = match path.file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => return,
    };

    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let sha256 = file_hash(&path).unwrap_or_default();
    let id = Uuid::new_v4().to_string();

    send_tcp_message(
        &cfg,
        &Message::DropStart {
            id: id.clone(),
            name,
            size,
            sha256,
        },
    )
    .await;

    let mut file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut buf = vec![0u8; 512 * 1024];

    loop {
        let read = match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };

        let data_b64 = general_purpose::STANDARD.encode(&buf[..read]);

        send_tcp_message(
            &cfg,
            &Message::DropChunk {
                id: id.clone(),
                data_b64,
            },
        )
        .await;
    }

    send_tcp_message(&cfg, &Message::DropEnd { id }).await;
}

pub fn receive_drop_start(name: String) -> PathBuf {
    let base = dirs_downloads().join("WinFlow Drops");
    let _ = fs::create_dir_all(&base);
    base.join(name)
}

pub fn append_drop_chunk(path: &Path, data_b64: String) {
    use std::io::Write;

    if let Ok(data) = general_purpose::STANDARD.decode(data_b64) {
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = file.write_all(&data);
        }
    }
}

fn file_hash(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 1024];

    loop {
        let n = file.read(&mut buf).ok()?;

        if n == 0 {
            break;
        }

        hasher.update(&buf[..n]);
    }

    Some(format!("{:x}", hasher.finalize()))
}

fn dirs_downloads() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join("Downloads"))
        .unwrap_or_else(|_| PathBuf::from("."))
}