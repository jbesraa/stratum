use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::sleep;

#[derive(Debug)]
pub struct DownstreamRole {
    addr: SocketAddr,
    sender_outgoing_data: Option<mpsc::Sender<Vec<u8>>>,
    runtime: Arc<RwLock<Option<Arc<tokio::runtime::Runtime>>>>,
    stop_me: tokio::sync::watch::Sender<()>,
}

impl DownstreamRole {
    pub fn new(addr: SocketAddr) -> Self {
        let (stop_me, _) = tokio::sync::watch::channel(());
        Self {
            runtime: Arc::new(RwLock::new(None)),
            addr,
            sender_outgoing_data: None,
            stop_me,
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let (sender_incoming_data, mut receiver_incoming_data) = mpsc::channel(1024);
        let (sender_outgoing_data, mut receiver_outgoing_data) = mpsc::channel(1024);
        self.sender_outgoing_data = Some(sender_outgoing_data.clone());

        let mut self_runtime = self.runtime.write().unwrap();
        if self_runtime.is_some() {
            return Err("Runtime already set".into());
        }

        let addr = self.addr.clone();
        let mut stop_listening = self.stop_me.subscribe();
        let stream = tokio::task::block_in_place(|| {
            runtime.block_on(async { TcpStream::connect(addr.clone()).await.unwrap() })
        });
        let (mut reader, mut writer) = stream.into_split();
        runtime.spawn(async move {
            loop {
                tokio::select!(
                    _ = stop_listening.changed() => {
                        println!("Stopping listener");
                        return;
                    },
                    _ = async {
                        if let Some(data) = receiver_outgoing_data.recv().await {
                            if let Err(e) = writer.write_all(&data).await {
                                eprintln!("Error sending data: {}", e);
                            }
                            println!("Sent data to upstream: {:?}", String::from_utf8_lossy(&data));
                        }
                    } => {}
                );
            }
        });

        let mut stop_reader = self.stop_me.subscribe();
        runtime.spawn(async move {
            loop {
                tokio::select!(
                    _ = stop_reader.changed() => {
                        println!("Stopping listener");
                        return;
                    },
                    _ = async {
                        let mut buf = Vec::<u8>::new();
                        let _ = reader.readable().await;
                        if let Err(e) = reader.read_to_end(&mut buf).await {
                            dbg!("Error reading response: {}", e);
                        } else {
                            if !buf.is_empty() {
                                dbg!("Received response: {}", String::from_utf8_lossy(&buf));
                                sender_incoming_data.send(buf).await.unwrap();
                            }
                        }
                        sleep(std::time::Duration::from_secs(3)).await;
                    } => {}
                );
            }
        });

        let mut stop_receiver = self.stop_me.subscribe();
        // let stop_rt = self.stop_me.clone();
        runtime.spawn(async move {
            tokio::select!(
                _ = stop_receiver.changed() => {
                    println!("Stopping receiver");
                    return;
                },
                _ = async {
                    while let Some(data) = receiver_incoming_data.recv().await {
                        println!(
                            "Received data from downstream: {:?}",
                            String::from_utf8_lossy(&data)
                        );
                        // let a = stop_rt.send(());
                    }
                } => {}
            );
        });

        *self_runtime = Some(Arc::new(runtime));
        Ok(())
    }

    pub fn send_data(&self, data: Vec<u8>) -> Result<(), Box<dyn Error>> {
        if let Some(sender) = self.sender_outgoing_data.as_ref() {
            self.runtime
                .read()
                .unwrap()
                .clone()
                .unwrap()
                .block_on(async {
                    if let Err(e) = sender.send(data).await {
                        eprintln!("Error sending data: {}", e);
                    }
                });
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let mut upstream_role = DownstreamRole::new(addr);
    upstream_role.start()?; // Start the upstream role

    upstream_role.send_data("Hello from downstream".as_bytes().to_vec())?;

    let runtime = upstream_role.runtime.read().unwrap().clone().unwrap();
    runtime.block_on(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if upstream_role
                .sender_outgoing_data
                .as_ref()
                .map_or(true, |s| s.is_closed())
            {
                break;
            }
        }
        println!("Main task done.");
    });

    Ok(())
}
