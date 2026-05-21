use crate::config::Config;
use crate::network::send_tcp_message;
use crate::protocol::Message;
use arboard::Clipboard;
use sha2::{Digest, Sha256};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;
pub fn start_clipboard_sync(
    cfg: Config,
    connected: Arc<AtomicBool>,
    last_remote_hash: Arc<Mutex<String>>,
) {
    if !cfg.clipboard_enabled {
        return;
    }
    let rt_handle = tokio::runtime::Handle::current();

    thread::spawn(move || {
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[clipboard] Failed to open clipboard: {e}");
                return;
            }
        };

        let mut last_local_hash = String::new();

        loop {
            if !connected.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(200));
                continue;
            }

            if let Ok(text) = clipboard.get_text() {
                let hash = hash_text(&text);
                let remote_hash = match last_remote_hash.lock() {
                    Ok(guard) => guard.clone(),
                    Err(poisoned) => poisoned.into_inner().clone(),
                };

                if hash != last_local_hash && hash != remote_hash {
                    last_local_hash = hash.clone();

                    let cfg_clone = cfg.clone();
                    let msg = Message::ClipboardText { text, hash };
                    rt_handle.spawn(async move {
                        send_tcp_message(&cfg_clone, &msg).await;
                    });
                }
            }

            thread::sleep(Duration::from_millis(500));
        }
    });
}
pub fn apply_remote_clipboard(
    text: String,
    hash: String,
    last_remote_hash: Arc<Mutex<String>>,
) {
    match last_remote_hash.lock() {
        Ok(mut guard) => *guard = hash,
        Err(poisoned) => *poisoned.into_inner() = hash,
    }

    match Clipboard::new() {
        Ok(mut clipboard) => {
            if let Err(e) = clipboard.set_text(text) {
                eprintln!("[clipboard] Failed to set clipboard text: {e}");
            }
        }
        Err(e) => eprintln!("[clipboard] Failed to open clipboard: {e}"),
    }
}
fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}