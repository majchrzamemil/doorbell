use std::{thread, time::Duration};

use futures_util::StreamExt;
use rusty_audio::Audio;
use tokio_tungstenite::connect_async;

const SERVER: &str = "ws://192.168.1.162:3000/doorbell";

#[tokio::main]
async fn main() {
    spawn_client().await
}

async fn spawn_client() {
    let mut reconnection_count: usize = 0;
    while reconnection_count < 1000 {
        println!("Connecting, reconnection_count :{reconnection_count}");
        let ws_stream = match connect_async(SERVER).await {
            Ok((stream, response)) => {
                println!("Handshake for client  has been completed");
                println!("Server response was {response:?}");
                reconnection_count = 0;
                stream
            }
            Err(e) => {
                println!("WebSocket handshake for client  failed with {e}!");
                reconnection_count += 1;
                thread::sleep(Duration::from_millis(1000));
                continue;
            }
        };

        let (_, mut receiver) = ws_stream.split();

        let mut audio = Audio::new();
        audio.add("startup", "./client/doorbell-223669.mp3");
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                tokio_tungstenite::tungstenite::Message::Binary(vec) => match &vec[..] {
                    [0] => {
                        println!("Tosia!!!!");
                        audio.play("startup");
                        audio.wait();
                    }
                    _ => {
                        println!("error");
                    }
                },
                tokio_tungstenite::tungstenite::Message::Close(_) => {
                    println!("closed connection");
                }
                tokio_tungstenite::tungstenite::Message::Ping(_) => {
                    //don't panic just ignore
                }
                _ => panic!("unexpected message"),
            }
        }
    }
}
