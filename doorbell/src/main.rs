use axum::extract::ws::Message;
use axum::{extract::ConnectInfo, response::IntoResponse, routing::any, Router};
use rppal::gpio::{Gpio, Level};
use std::error::Error;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tower_http::trace::{DefaultMakeSpan, TraceLayer};
// Gpio uses BCM pin numbering. BCM GPIO 24 is tied to physical pin 16.
const GPIO_SENSOR: u8 = 23;

//API definition
//[0] - doorbell
//[1] - error

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let app = Router::new()
        .route("/doorbell", any(ws_handler))
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::debug!("listening on {}", listener.local_addr()?);

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
    tracing::info!("Connected client: {who}");

    let Ok(gpio) = Gpio::new() else {
        tracing::error!("Unable to init GPIO");
        send_message(&mut socket, Message::Binary(vec![1])).await;
        return;
    };

    let Ok(pin) = gpio.get(GPIO_SENSOR) else {
        tracing::error!("Unable to get GPIO pin {GPIO_SENSOR}");
        send_message(&mut socket, Message::Binary(vec![1])).await;
        return;
    };
    let pin = pin.into_input_pullup();

    loop {
        if pin.read() == Level::Low {
            tracing::info!("doorbell");
            if send_message(&mut socket, Message::Binary(vec![0]))
                .await
                .is_none()
            {
                break;
            }
            thread::sleep(Duration::from_millis(2500));
        } else {
            thread::sleep(Duration::from_millis(500));
        }
    }
    tracing::info!("Closing connection with: {who}");
}

async fn send_message(socket: &mut axum::extract::ws::WebSocket, msg: Message) -> Option<()> {
    match socket.send(msg).await {
        Ok(_) => tracing::info!("message sent successfuly"),
        Err(e) => {
            tracing::error!(err = ?e, "Encountered error while sending to socket");
            return None;
        }
    }
    Some(())
}
