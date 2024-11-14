use axum::extract::ws::Message;
use axum::extract::State;
use axum::{extract::ConnectInfo, response::IntoResponse, routing::any, Router};
use rppal::gpio::{Gpio, Level};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{io, thread};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tower_http::trace::{DefaultMakeSpan, TraceLayer};
// Gpio uses BCM pin numbering. BCM GPIO 24 is tied to physical pin 16.
const GPIO_SENSOR: u8 = 23;

//API definition
//[0] - doorbell
//[1] - error

#[derive(Clone)]
struct AppState {
    gpio_state: Arc<AtomicBool>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_line_number(true)
                .with_file(true)
                .with_thread_names(true)
                .with_thread_ids(true),
        )
        .init();

    let gpio_state = Arc::new(AtomicBool::new(false));
    let state = AppState {
        gpio_state: gpio_state.clone(),
    };
    let app = Router::new()
        .route("/doorbell", any(ws_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::debug!("listening on {}", listener.local_addr()?);

    let res = futures::future::try_join_all(vec![
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
        }),
        tokio::spawn(async move { gpio_hander(gpio_state.clone()) }),
    ])
    .await;

    tracing::info!("Finished");
    match res {
        Ok(results) => {
            for result in results.into_iter() {
                if let Err(e) = result {
                    tracing::error!(error = %e);
                }
            }
        }
        Err(err) => tracing::error!(error = %err),
    }
    Ok(())
}

fn gpio_hander(state: Arc<AtomicBool>) -> Result<(), io::Error> {
    let Ok(gpio) = Gpio::new() else {
        tracing::error!("Unable to init GPIO");
        return Ok(());
    };

    let Ok(pin) = gpio.get(GPIO_SENSOR) else {
        tracing::error!("Unable to get GPIO pin {GPIO_SENSOR}");
        return Ok(());
    };
    let pin = pin.into_input_pullup();
    tracing::info!("Successfuly initialized GPIO pin {GPIO_SENSOR}");

    loop {
        if pin.read() == Level::Low {
            if !state.load(Ordering::Acquire) {
                state.store(true, Ordering::Release);
                tracing::info!("State updated to true");
            }
        } else {
            if state.load(Ordering::Acquire) {
                state.store(false, Ordering::Release);
                tracing::info!("State updated to false");
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
}

async fn ws_handler(
    ws: axum::extract::WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("WS handler");
    let upgrade = ws.on_upgrade(move |socket| handle_socket(socket, addr, state.gpio_state));
    tracing::info!("upgrade finished");
    upgrade
}

async fn handle_socket(
    mut socket: axum::extract::ws::WebSocket,
    who: SocketAddr,
    gpio_state: Arc<AtomicBool>,
) {
    tracing::info!("Connected client: {who}");

    loop {
        if gpio_state.load(Ordering::Acquire) {
            tracing::info!("doorbell");
            if send_message(&mut socket, Message::Binary(vec![0]))
                .await
                .is_none()
            {
                break;
            }
            tracing::info!("Message successfuly sent");
            thread::sleep(Duration::from_millis(2500));
        } else {
            if send_message(&mut socket, Message::Ping(vec![]))
                .await
                .is_none()
            {
                tracing::warn!("Client disconnected");
                break;
            }
            thread::sleep(Duration::from_millis(1000));
        }
    }
    tracing::info!("Closing connection with: {who}");
}

async fn send_message(socket: &mut axum::extract::ws::WebSocket, msg: Message) -> Option<()> {
    if let Err(e) = socket.send(msg).await {
        tracing::error!(err = ?e, "Encountered error while sending to socket");
        return None;
    }
    Some(())
}
