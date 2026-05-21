mod clipboard;
mod config;
mod cursor_switch;
mod discovery;
mod file_drop;
mod input_capture;
mod input_inject;
mod network;
mod protocol;
mod mouse_hook;

use crate::clipboard::{apply_remote_clipboard, start_clipboard_sync};
use crate::config::{load_config, save_config};
use crate::cursor_switch::start_cursor_switch;
use crate::input_inject::{
    inject_mouse_button,
    inject_mouse_move,
    inject_mouse_wheel,
    place_cursor_from_edge,
};
use crate::network::{send_tcp_message, start_tcp_server, start_udp_server};
use crate::protocol::Message;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let mut cfg = load_config();

    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 && args[1] == "--connect" {
        cfg.peer_ip = args[2].clone();
        save_config(&cfg);
    } else {
        println!("WINFLOW_WAITING_FOR_UI");
        println!("Start from ui.py and press Connect.");
        return;
    }

    println!("WINFLOW_STARTING");
    println!("Device: {}", cfg.device_name);
    println!("Peer: {}", cfg.peer_ip);

    let connected = Arc::new(AtomicBool::new(false));
    let remote_mode = Arc::new(AtomicBool::new(false));
    let last_remote_clipboard_hash = Arc::new(Mutex::new(String::new()));
    let active_drops: Arc<Mutex<HashMap<String, PathBuf>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let (incoming_tx, mut incoming_rx) = mpsc::channel::<Message>(1024);

    start_udp_server(cfg.clone(), incoming_tx.clone());

    tokio::spawn(start_tcp_server(
        cfg.clone(),
        incoming_tx.clone(),
    ));

    discovery::start_discovery(cfg.clone());

    start_cursor_switch(cfg.clone(), connected.clone(), remote_mode.clone());
    mouse_hook::start_mouse_hook(
    cfg.clone(),
    connected.clone(),
    remote_mode.clone(),
    );
    start_clipboard_sync(
        cfg.clone(),
        connected.clone(),
        last_remote_clipboard_hash.clone(),
    );

    let cfg_ping = cfg.clone();
    let connected_ping = connected.clone();

    tokio::spawn(async move {
        loop {
            let ok = send_tcp_message(&cfg_ping, &Message::Ping).await;

            if !ok {
                connected_ping.store(false, Ordering::Relaxed);
                println!("WINFLOW_DISCONNECTED");
            }

            sleep(Duration::from_secs(1)).await;
        }
    });

    while let Some(msg) = incoming_rx.recv().await {
        match msg {
            Message::EnterControl { edge } => {
                    remote_mode.store(false, Ordering::Relaxed);
                    place_cursor_from_edge(edge);
                    println!("WINFLOW_CURSOR_RECEIVED");
            }

            Message::ExitControl => {
                remote_mode.store(false, Ordering::Relaxed);
            }
            Message::ReturnControl => {
                remote_mode.store(false, Ordering::Relaxed);
                println!("WINFLOW_CURSOR_RETURNED");
            }

            Message::OpenUrl { url } => {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &url])
                    .spawn();
            }

            Message::MouseMove { dx, dy } => {
                inject_mouse_move(dx, dy);
            }

            Message::MouseButton { button, down } => {
                inject_mouse_button(button, down);
            }
            Message::MouseWheel { delta } => {
                inject_mouse_wheel(delta);
            }

            Message::ClipboardText { text, hash } => {
                apply_remote_clipboard(
                    text,
                    hash,
                    last_remote_clipboard_hash.clone(),
                );
            }

            Message::DropStart { id, name, .. } => {
                let path = file_drop::receive_drop_start(name);

                if let Ok(mut drops) = active_drops.lock() {
                    drops.insert(id, path);
                }
            }

            Message::DropChunk { id, data_b64 } => {
                if let Ok(drops) = active_drops.lock() {
                    if let Some(path) = drops.get(&id) {
                        file_drop::append_drop_chunk(path, data_b64);
                    }
                }
            }

            Message::DropEnd { id } => {
                if let Ok(mut drops) = active_drops.lock() {
                    drops.remove(&id);
                }
            }

            Message::Ping => {
                connected.store(true, Ordering::Relaxed);
                send_tcp_message(&cfg, &Message::Pong).await;
            }

            Message::Pong => {
                connected.store(true, Ordering::Relaxed);
                println!("WINFLOW_CONNECTED");
            }

            Message::Hello(device) => {
                println!("Discovered device: {}", device.name);
            }
        }
    }
}