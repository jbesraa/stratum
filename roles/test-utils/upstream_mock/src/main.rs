use binary_sv2::{Deserialize, GetSize, Serialize};
use codec_sv2::{HandShakeFrame, HandshakeRole, StandardEitherFrame, StandardNoiseDecoder};
use const_sv2::RESPONDER_EXPECTED_HANDSHAKE_MESSAGE_SIZE;
use key_utils::{Secp256k1PublicKey, Secp256k1SecretKey};
use noise_sv2::Responder;
use roles_logic_sv2::parsers::AnyMessage;
use std::{
    error::Error,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::mpsc,
    time::sleep,
};

#[derive(Debug)]
pub struct UpstreamRole {
    upstream_addr: SocketAddr,
    state: Arc<tokio::sync::RwLock<codec_sv2::State>>,
    runtime: Arc<RwLock<Option<Arc<tokio::runtime::Runtime>>>>,
    stop_me: tokio::sync::watch::Sender<()>,
}

impl UpstreamRole {
    pub fn new(upstream_addr: SocketAddr) -> Self {
        let (stop_me, _) = tokio::sync::watch::channel(());
        let pub_key = "9auqWEzQDVyd2oe1JVGFLMLHZtCo2FFqZwtKA5gd9xbuEu7PH72"
            .to_string()
            .parse::<Secp256k1PublicKey>()
            .unwrap()
            .into_bytes();
        let prv_key = "mkDLTBBRxdBv998612qipDYoTK3YUrqLe8uWw7gu3iXbSrn2n"
            .to_string()
            .parse::<Secp256k1SecretKey>()
            .unwrap()
            .into_bytes();
        let responder =
            Responder::from_authority_kp(&pub_key, &prv_key, std::time::Duration::from_secs(10000))
                .unwrap();
        Self {
            state: Arc::new(tokio::sync::RwLock::new(codec_sv2::State::not_initialized(
                &HandshakeRole::Responder(responder),
            ))),
            runtime: Arc::new(RwLock::new(None)),
            upstream_addr,
            stop_me,
        }
    }

    pub fn start<'a, Message: Serialize + Deserialize<'a> + GetSize + Send + 'static>(
        &mut self,
    ) -> Result<(), Box<dyn Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let (sender_outgoing_data, mut receiver_incoming_data) =
            mpsc::channel::<StandardEitherFrame<AnyMessage<'static>>>(1024);

        let (sender_incoming_data, mut receiver_outgoing_data) =
            mpsc::channel::<StandardEitherFrame<AnyMessage<'static>>>(1024);

        // Acquire the lock and check if the runtime is already set.
        let mut self_runtime = self.runtime.write().unwrap();
        if self_runtime.is_some() {
            return Err("Runtime already set".into());
        }
        let upstream_addr = self.upstream_addr.clone();
        let tcp = tokio::task::block_in_place(|| {
            runtime.block_on(async { TcpListener::bind(upstream_addr.clone()).await.unwrap() })
        });
        println!("Upstream accepting connections on: {}", upstream_addr);
        let original_state = self.state.clone();
        let original_state_1 = self.state.clone();
        let original_state_2 = self.state.clone();
        let mut stop_reader_writer = self.stop_me.subscribe();
        let sender_outgoing_data = sender_outgoing_data.clone();
        runtime.spawn(async move {
            loop {
                let (stream, _addr) = tcp.accept().await.unwrap();
                let (mut reader, mut writer) = stream.into_split();
                let mut decoder = StandardNoiseDecoder::<AnyMessage<'static>>::new();
                let mut encoder = codec_sv2::NoiseEncoder::<AnyMessage<'static>>::new();
                tokio::select!(
                _ = stop_reader_writer.changed() => { }
                _ = async {
                  loop {
                    if let Some(m) = receiver_incoming_data.recv().await{
                    let state = original_state.clone();
                    let mut state = state.write().await;
                    dbg!(&state);
                    dbg!(&m);
                    let b = encoder.encode(m.into(), &mut state).unwrap();
                    let b = b.as_ref();
                    let _ = writer.write_all(b).await;
                    dbg!("Sent data");
                  }
                  }
				} => {}
                  _ = async {
                  loop {
                    let writable = decoder.writable();
                    if let Ok(_) =  reader.read_exact(writable).await {
                      let state = original_state_1.clone();
                      let mut state = state.write().await;
                      dbg!(&state);
                      match decoder.next_frame(&mut state) {
                        Ok(x) => {
                          dbg!(&x);
                          let _ = sender_incoming_data.send(x.into()).await;
                        },
                        Err(e) => {
                          dbg!("error: {}", e);
                        }
                      };
                    }
                  }
                } => {}
                );
            }
        });
        runtime.spawn(async move {
            let responder = Responder::from_authority_kp(
                &"9auqWEzQDVyd2oe1JVGFLMLHZtCo2FFqZwtKA5gd9xbuEu7PH72"
                    .to_string()
                    .parse::<Secp256k1PublicKey>()
                    .unwrap()
                    .into_bytes(),
                &"mkDLTBBRxdBv998612qipDYoTK3YUrqLe8uWw7gu3iXbSrn2n"
                    .to_string()
                    .parse::<Secp256k1SecretKey>()
                    .unwrap()
                    .into_bytes(),
                std::time::Duration::from_secs(10000),
            )
            .unwrap();
            let role = HandshakeRole::Responder(responder);
            let mut initial_state = codec_sv2::State::initialized(role);
            dbg!("Initialized state");
            let decoded = receiver_outgoing_data.recv().await.unwrap();
            let first_message: HandShakeFrame = decoded.try_into().unwrap();
            dbg!("Received first message");
            let first_message: [u8; RESPONDER_EXPECTED_HANDSHAKE_MESSAGE_SIZE] = first_message
                .get_payload_when_handshaking()
                .try_into()
                .unwrap();

            dbg!("Before sending second message");
            // Create and send second handshake message
            let (second_message, transport_mode) = initial_state.step_1(first_message).unwrap();

            let res = sender_outgoing_data
                .send(second_message.into())
                .await
                .unwrap();
            dbg!(&res);
            dbg!("After sending second message");

            sleep(Duration::from_secs(3)).await;
            let mut ori_state = original_state_2.write().await;
            *ori_state = transport_mode;
        });

        *self_runtime = Some(Arc::new(runtime));
        Ok(())
    }

    pub fn shutdown(&self) {
        self.stop_me.send(()).unwrap();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let upstream_addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let mut upstream_role = UpstreamRole::new(upstream_addr);
    upstream_role.start::<AnyMessage>()?; // Start the upstream role

    let runtime = upstream_role.runtime.read().unwrap().clone().unwrap();
    runtime.block_on(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    Ok(())
}
