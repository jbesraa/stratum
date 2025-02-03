use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct UpstreamRole {
    addr: SocketAddr,
    sender: Option<mpsc::Sender<Vec<u8>>>,
    runtime: Arc<RwLock<Option<Arc<tokio::runtime::Runtime>>>>,
    stop_me: tokio::sync::watch::Sender<()>,
}

impl UpstreamRole {
    pub fn new(addr: SocketAddr) -> Self {
        let (stop_me, _) = tokio::sync::watch::channel(());
        Self {
            runtime: Arc::new(RwLock::new(None)),
            addr,
            sender: None,
            stop_me,
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let (send, mut receive) = mpsc::channel(1024);
        self.sender = Some(send.clone());

        let mut self_runtime = self.runtime.write().unwrap();
        if self_runtime.is_some() {
            return Err("Runtime already set".into());
        }

        let cloned_tx = send.clone();
        let addr = self.addr.clone();

        let mut stop_listening = self.stop_me.subscribe();
        runtime.spawn(async move {
            let listener = TcpListener::bind(addr.clone()).await.unwrap();
            println!("UpstreamRole listening on {}", addr);
            loop {
                tokio::select!(
                    _ = stop_listening.changed() => {
                        println!("Stopping listener");
                        return;
                    },
                    _ = async {
                        let (stream, addr) = listener.accept().await.unwrap();
                        let (mut reader, mut writer) = stream.into_split();
                        println!("Accepted connection from {}", addr);
                        let mut buf = vec![0; 1024];
                        let n = reader.read(&mut buf).await.unwrap();
                        let data = &buf[..n];

                        if let Err(e) = cloned_tx.send(data.to_vec()).await {
                            println!("Error sending data: {}", e);
                        } else {
                            println!("Data sent");
                        }
                        let d = "Hello from upstream".as_bytes();
                        let res = writer.write_all(d).await.unwrap();
                        dbg!(&res);
                    } => {}
                );
            }
        });

        let mut stop_receiver = self.stop_me.subscribe();
        // let stop_rt = self.stop_me.clone();
        runtime.spawn(async move {
            loop {
                tokio::select!(
                    _ = stop_receiver.changed() => {
                        println!("Stopping receiver");
                        return;
                    },
                    _ = async {
                        while let Some(data) = receive.recv().await {
                            println!(
                                "Received data from downstream: {:?}",
                                String::from_utf8_lossy(&data)
                            );
                            // let a = stop_rt.send(());
                        }
                    } => {}
                );
            }
        });

        *self_runtime = Some(Arc::new(runtime));
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let mut upstream_role = UpstreamRole::new(addr);
    upstream_role.start()?; // Start the upstream role

    let runtime = upstream_role.runtime.read().unwrap().clone().unwrap(); // Get the runtime

    // Block the main thread using runtime.block_on
    runtime.block_on(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await; // Small delay to avoid busy-looping
            if upstream_role
                .sender
                .as_ref()
                .map_or(true, |s| s.is_closed())
            {
                break; // Exit the loop when the sender is closed
            }
        }
        println!("Main task done."); // This will print when the sender is dropped.
    });

    Ok(())
}
