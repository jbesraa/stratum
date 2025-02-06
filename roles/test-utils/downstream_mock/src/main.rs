use codec_sv2::{
    HandShakeFrame, HandshakeRole, StandardEitherFrame, StandardNoiseDecoder, Sv2Frame,
};
use const_sv2::{INITIATOR_EXPECTED_HANDSHAKE_MESSAGE_SIZE, MESSAGE_TYPE_SETUP_CONNECTION};
use noise_sv2::Initiator;
use roles_logic_sv2::{
    common_messages_sv2::{Protocol, SetupConnection},
    parsers::{AnyMessage, CommonMessages, PoolMessages},
};
use std::{error::Error, net::SocketAddr, sync::Arc, thread::sleep, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};

#[derive(Debug)]
pub struct DownstreamRole {
    upstream_addr: SocketAddr,
    sender_outgoing_data: Option<mpsc::Sender<StandardEitherFrame<AnyMessage<'static>>>>,
    state: Arc<tokio::sync::RwLock<codec_sv2::State>>,
    runtime: Arc<std::sync::RwLock<Option<Arc<tokio::runtime::Runtime>>>>,
    stop_me: tokio::sync::watch::Sender<()>,
}

impl DownstreamRole {
    pub fn new(upstream_addr: SocketAddr) -> Self {
        let (stop_me, _) = tokio::sync::watch::channel(());
        Self {
            runtime: Arc::new(std::sync::RwLock::new(None)),
            upstream_addr,
            state: Arc::new(tokio::sync::RwLock::new(codec_sv2::State::not_initialized(
                &HandshakeRole::Initiator(Initiator::without_pk().unwrap()),
            ))),
            sender_outgoing_data: None,
            stop_me,
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        // Channel for receiving data from the upstream. `sender_incoming_data` is used to send
        // data to the channel and `receiver_incoming_data` is continuously listening for data to
        // receive in an async loop.
        let (sender_incoming_data, mut receiver_outgoing_data) =
            tokio::sync::mpsc::channel::<StandardEitherFrame<AnyMessage<'static>>>(1024);
        // Channel for sending data to the upstream. `sender_outgoing_data` is used to send data to
        // to the channel and `receiver_outgoing_data` is continuously listening for data to send
        // in an async loop.
        let (sender_outgoing_data, mut receiver_incoming_data) =
            mpsc::channel::<StandardEitherFrame<AnyMessage<'static>>>(1024);

        self.sender_outgoing_data = Some(sender_outgoing_data.clone());

        // Acquire the lock and check if the runtime is already set.
        let mut self_runtime = self.runtime.write().unwrap();
        if self_runtime.is_some() {
            return Err("Runtime already set".into());
        }
        let upstream_addr = self.upstream_addr.clone();
        let stream = tokio::task::block_in_place(|| {
            runtime.block_on(async { TcpStream::connect(upstream_addr).await.unwrap() })
        });

        // Split the stream into reader and writer.
        let (mut reader, mut writer) = stream.into_split();

        let original_state = self.state.clone();
        let original_state_1 = self.state.clone();
        let original_state_2 = self.state.clone();
        let mut stop_reader_writer = self.stop_me.subscribe();
        runtime.spawn(async move {
            let mut decoder = StandardNoiseDecoder::<AnyMessage<'static>>::new();
            let mut encoder = codec_sv2::NoiseEncoder::<AnyMessage<'static>>::new();
            loop {
                tokio::select!(
                  _ = stop_reader_writer.changed() => {
                    break;
                  }
                  _ = async {
                      loop {
                          if let Some(m) = receiver_incoming_data.recv().await{
                          dbg!("here");
                          dbg!(&m);
                          let state = original_state.clone();
                          let mut state = state.write().await;
                          dbg!(&state);
                          let b = encoder.encode(m.into(), &mut state).unwrap();
                          let b = b.as_ref();
                          let _ = writer.write_all(b).await;
                          dbg!("Sent data");
                      }
                      }
                  } => {}
                  _ = async {
                      loop
                      {
                          let writable = decoder.writable();
                          if let Ok(_) =  reader.read_exact(writable).await {
                              let state = original_state_1.clone();
                              let mut state = state.write().await;
                              match decoder.next_frame(&mut state) {
                                  Ok(x) => {
                                      let _ = sender_incoming_data.send(x.into()).await;
                                  },
                                  Err(e) => {
                                      dbg!("error: {}", e);
                                  }
                              };
                          }
                      }                  } => {}
                );
            }
        });

        // Perform Handshake
        runtime.spawn(async move {
            let role = HandshakeRole::Initiator(Initiator::without_pk().unwrap());
            let mut initialized_state = codec_sv2::State::initialized(role);
            let first_message = initialized_state.step_0().unwrap();
            sender_outgoing_data
                .send(first_message.into())
                .await
                .unwrap();
            let decoded = receiver_outgoing_data.recv().await.unwrap();
            let second_message: HandShakeFrame = decoded.try_into().unwrap();
            let second_message: [u8; INITIATOR_EXPECTED_HANDSHAKE_MESSAGE_SIZE] = second_message
                .get_payload_when_handshaking()
                .try_into()
                .unwrap();
            let transport_mode = initialized_state.step_2(second_message).unwrap();
            let mut ori_state = original_state_2.write().await;
            sleep(Duration::from_secs(3));
            *ori_state = transport_mode;
            dbg!(&ori_state);
        });

        *self_runtime = Some(Arc::new(runtime));
        Ok(())
    }

    pub fn send_data(
        &self,
        data: StandardEitherFrame<AnyMessage<'static>>,
    ) -> Result<(), Box<dyn Error>> {
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

    pub fn shutdown(&self) {
        self.stop_me.send(()).unwrap();
    }
}

fn get_setup_connection_message(
    min_version: u16,
    max_version: u16,
    is_work_selection_enabled: bool,
    protocol: Protocol,
) -> SetupConnection<'static> {
    let endpoint_host = "0.0.0.0".to_string().into_bytes().try_into().unwrap();
    let vendor = String::new().try_into().unwrap();
    let hardware_version = String::new().try_into().unwrap();
    let firmware = String::new().try_into().unwrap();
    let device_id = String::new().try_into().unwrap();
    let flags = match is_work_selection_enabled {
        false => 0b0000_0000_0000_0000_0000_0000_0000_0100,
        true => 0b0000_0000_0000_0000_0000_0000_0000_0110,
    };
    SetupConnection {
        protocol,
        min_version,
        max_version,
        flags,
        endpoint_host,
        endpoint_port: 50,
        vendor,
        hardware_version,
        firmware,
        device_id,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let upstream_addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let mut downstream_role = DownstreamRole::new(upstream_addr);
    downstream_role.start()?;

    sleep(Duration::from_secs(2));
    let setup_connection_message =
        get_setup_connection_message(2, 2, false, Protocol::JobDeclarationProtocol);
    let any_message =
        PoolMessages::Common(CommonMessages::SetupConnection(setup_connection_message));
    let sv2_frame =
        Sv2Frame::from_message(any_message, MESSAGE_TYPE_SETUP_CONNECTION, 0, false).unwrap();
    let frame = StandardEitherFrame::<AnyMessage>::Sv2(sv2_frame);
    downstream_role.send_data(frame)?;

    let runtime = downstream_role.runtime.read().unwrap().clone().unwrap();
    runtime.block_on(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if downstream_role
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
