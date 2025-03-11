use std::sync::Arc;
// use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::signal::unix::{signal, SignalKind};
use tokio::{
    net::TcpListener,
    sync::{mpsc, watch, Mutex},
};
use tracing::{debug, error, info};

#[derive(Debug)]
pub enum ServerError {
    IoError(std::io::Error),
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        ServerError::IoError(e)
    }
}

pub struct GenericTcpServer {
    id: u32,
    name: String,
    state: ServerState,
}

#[derive(Clone, Debug)]
pub enum ServerState {
    Stopped,
    Running,
}

impl GenericTcpServer {
    pub async fn new<H>(handler: H, mut role_shutdown_signal: watch::Receiver<()>) -> Self
    where
        H: UpstreamServerRole + Send + 'static,
    {
        // Create a watch channel to monitor the server state
        let (tx, mut rx) = watch::channel(ServerState::Stopped);
        let tx = Arc::new(Mutex::new(tx));
        // Create an mpsc channel for graceful shutdown
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        // Task to handle the TCP server
        let server_task = tokio::task::spawn({
            let tx = Arc::clone(&tx);
            async move { Self::server_task(tx, handler).await }
        });
        // Spawn the task to monitor the state
        let monitor_task = tokio::task::spawn({
            let tx = Arc::clone(&tx);
            async move { monitor_state(tx).await }
        });
        // Handle graceful shutdown on SIGINT or SIGTERM
        tokio::task::spawn({
            let shutdown_tx = shutdown_tx.clone();
            async move { monitor_sigint_sigterm(shutdown_tx).await }
        });
        tokio::task::spawn({
            async move {
                tokio::select! {
                  _ = role_shutdown_signal.changed() => {},
                    _ = server_task => {},
                    _ = monitor_task => {},
                    _ = shutdown_rx.recv() => {
                        info!("Shutting down server...");
                    }
                };
            }
        });
        return Self {
            id: 0,
            name: "tcp_server".to_string(),
            state: rx.borrow_and_update().clone(),
        };
    }

    async fn server_task<H>(
        tx: Arc<Mutex<watch::Sender<ServerState>>>,
        handler: H,
    ) -> Result<(), ServerError>
    where
        H: UpstreamServerRole,
    {
        let listener = TcpListener::bind("127.0.0.1:8080").await?;
        info!("Server started on 127.0.0.1:8080");

        // Send the "Running" state to the watch channel
        let tx_send = tx.lock().await;
        tx_send.send(ServerState::Running).unwrap();

        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    info!("New connection established");
                    tokio::task::spawn(handler.handle_connection(socket, Arc::clone(&tx)));
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

pub trait UpstreamServerRole {
    fn handle_connection(
        &self,
        socket: TcpStream,
        tx: Arc<Mutex<watch::Sender<ServerState>>>,
    ) -> tokio::task::JoinHandle<()>;
}

async fn monitor_state(tx: Arc<Mutex<watch::Sender<ServerState>>>) {
    loop {
        let state = tx.lock().await;
        debug!("Current server state: {:?}", state);
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

async fn monitor_sigint_sigterm(shutdown_tx: mpsc::Sender<()>) {
    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to listen to SIGINT");
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to listen to SIGTERM");
    tokio::select! {
        _ = sigint.recv() => {
            info!("Received SIGINT, initiating shutdown...");
        }
        _ = sigterm.recv() => {
            info!("Received SIGTERM, initiating shutdown...");
        }
    }
    // Signal shutdown to the server
    shutdown_tx.send(()).await.unwrap();
}

// async fn handle_connection(mut socket: TcpStream, tx: Arc<Mutex<watch::Sender<ServerState>>>) {
//     let mut buffer = vec![0; 1024];
//     loop {
//         match socket.read(&mut buffer).await {
//             Ok(0) => {
//                 info!("Connection closed");
//                 break;
//             }
//             Ok(n) => {
//                 let received = String::from_utf8_lossy(&buffer[..n]);
//                 info!("Received: {}", received);
//                 let tx = tx.lock().await;
//                 if let Err(e) = tx.send(ServerState::Running) {
//                     error!("Failed to send server state update: {:?}", e);
//                 }
//                 if let Err(e) = socket.write_all(b"Message received").await {
//                     error!("Failed to send response: {:?}", e);
//                 }
//             }
//             Err(e) => {
//                 error!("Failed to read from socket: {:?}", e);
//                 break;
//             }
//         }
//     }
// }
