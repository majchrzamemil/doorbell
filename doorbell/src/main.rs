use axum::extract::ws::Message;
use axum::{extract::ConnectInfo, response::IntoResponse, routing::any, Router};
use rppal::gpio::{Gpio, Level};
use std::error::Error;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

// Gpio uses BCM pin numbering. BCM GPIO 24 is tied to physical pin 16.
const GPIO_SENSOR: u8 = 23;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let app = Router::new().route("/doorbell", any(ws_handler));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    Ok(axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?)
}

async fn ws_handler(
    ws: axum::extract::WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}
async fn handle_socket(mut socket: axum::extract::ws::WebSocket, who: SocketAddr) {
    println!("Connected client: {who}");
    // Send error message on invalid init
    let gpio = Gpio::new().unwrap();
    let pin = gpio.get(GPIO_SENSOR).unwrap().into_input_pulldown();
    loop {
        if pin.read() == Level::Low {
            match socket.send(Message::Binary(vec![])).await {
                Ok(_) => println!("doorbell"),
                Err(e) => {
                    eprintln!("Encountered error:{e}");
                    break;
                }
            }
            thread::sleep(Duration::from_millis(2000));
        } else {
            thread::sleep(Duration::from_millis(300));
        }
    }
    println!("Closing connection with: {who}");
}
