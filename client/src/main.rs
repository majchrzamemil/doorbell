use futures_util::StreamExt;
use rusty_audio::Audio;
use tokio_tungstenite::connect_async;

const SERVER: &str = "ws://192.168.1.162:3000/doorbell";

#[tokio::main]
async fn main() {
    spawn_client().await
}

async fn spawn_client() {
    let ws_stream = match connect_async(SERVER).await {
        Ok((stream, response)) => {
            println!("Handshake for client  has been completed");
            println!("Server response was {response:?}");
            stream
        }
        Err(e) => {
            println!("WebSocket handshake for client  failed with {e}!");
            return;
        }
    };

    let (_, mut receiver) = ws_stream.split();

    let mut audio = Audio::new();
    audio.add(
        "startup",
        "/Users/emil.majchrzak/Documents/sound_test/doorbell-223669.mp3",
    );
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
            _ => panic!("unexpected message"),
        }
    }
}
