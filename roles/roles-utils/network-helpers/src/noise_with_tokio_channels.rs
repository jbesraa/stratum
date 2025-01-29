#[cfg(feature = "with_tokio")]
use crate::Error;
use binary_sv2::{Deserialize, Serialize};
use const_sv2::{
    INITIATOR_EXPECTED_HANDSHAKE_MESSAGE_SIZE, RESPONDER_EXPECTED_HANDSHAKE_MESSAGE_SIZE,
};
use std::{
    convert::TryInto,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{
        broadcast::{channel, Receiver, Sender},
        RwLock,
    },
    task::{self},
};

use binary_sv2::GetSize;
use codec_sv2::{HandShakeFrame, HandshakeRole, StandardEitherFrame, StandardNoiseDecoder};

use tracing::error;

#[derive(Debug, Clone)]
pub struct SV2ConnectionSetup<Message: Serialize + Deserialize<'static> + GetSize + Clone> {
    role: Arc<HandshakeRole>,
    state: Arc<RwLock<codec_sv2::State>>,
    sender_incoming: Sender<StandardEitherFrame<Message>>,
    sender_outgoing: Sender<StandardEitherFrame<Message>>,
}

impl<Message: Serialize + Deserialize<'static> + GetSize + Clone + Send + 'static>
    SV2ConnectionSetup<Message>
{
    pub fn new(role: HandshakeRole) -> Self {
        let state = Arc::new(RwLock::new(codec_sv2::State::not_initialized(&role)));
        let (sender_incoming, _receiver_incoming): (
            Sender<StandardEitherFrame<Message>>,
            Receiver<StandardEitherFrame<Message>>,
        ) = channel(10);
        let (sender_outgoing, _receiver_outgoing): (
            Sender<StandardEitherFrame<Message>>,
            Receiver<StandardEitherFrame<Message>>,
        ) = channel(10);
        SV2ConnectionSetup {
            role: Arc::new(role),
            state,
            sender_incoming,
            sender_outgoing,
        }
    }

    pub async fn start(
        &self,
        stream: TcpStream,
    ) -> Result<
        (
            Receiver<StandardEitherFrame<Message>>,
            Sender<StandardEitherFrame<Message>>,
        ),
        Error,
    > {
        let me = self.clone(); // maybe this is bad
        tokio::spawn(async move {
            me.handle_connection(stream).await;
        });
        let _ = self.handashake().await;
        let sender_outgoing = self.sender_outgoing.clone();
        let receiver_incoming = sender_outgoing.subscribe();
        Ok((receiver_incoming, sender_outgoing))
    }

    async fn handashake(
        &self,
    ) -> Result<
        (
            Receiver<StandardEitherFrame<Message>>,
            Sender<StandardEitherFrame<Message>>,
        ),
        Error,
    > {
        // DO THE NOISE HANDSHAKE
        match self.role.as_ref() {
            HandshakeRole::Initiator(_) => {
                // Create and send first handshake message
                let mut receiver_incoming = self.sender_outgoing.subscribe();
                let mut write_state = self.state.write().await;
                let first_message = write_state.step_0().unwrap();
                let _ = self.sender_outgoing.send(first_message.into());
                // Receive and deserialize second handshake message
                let second_message = receiver_incoming.recv().await.unwrap();
                let second_message: HandShakeFrame = second_message
                    .try_into()
                    .map_err(|_| Error::HandshakeRemoteInvalidMessage)
                    .unwrap();
                let second_message: [u8; INITIATOR_EXPECTED_HANDSHAKE_MESSAGE_SIZE] =
                    second_message
                        .get_payload_when_handshaking()
                        .try_into()
                        .map_err(|_| Error::HandshakeRemoteInvalidMessage)
                        .unwrap();
                // Create and send thirth handshake message
                let transport_mode = write_state.step_2(second_message).unwrap();
                self.set_state(transport_mode).await;
                while !TRANSPORT_READY.load(std::sync::atomic::Ordering::SeqCst) {
                    std::hint::spin_loop();
                }
            }
            HandshakeRole::Responder(_) => {
                // Receive and deserialize first handshake message
                let mut receiver_incoming = self.sender_outgoing.subscribe();
                let mut write_state = self.state.write().await;
                let first_message: HandShakeFrame = receiver_incoming
                    .recv()
                    .await
                    .unwrap()
                    .try_into()
                    .map_err(|_| Error::HandshakeRemoteInvalidMessage)
                    .unwrap();
                let first_message: [u8; RESPONDER_EXPECTED_HANDSHAKE_MESSAGE_SIZE] = first_message
                    .get_payload_when_handshaking()
                    .try_into()
                    .map_err(|_| Error::HandshakeRemoteInvalidMessage)
                    .unwrap();

                // Create and send second handshake message
                let (second_message, transport_mode) = write_state.step_1(first_message).unwrap();
                HANDSHAKE_READY.store(false, std::sync::atomic::Ordering::SeqCst);
                let _ = self.sender_outgoing.send(second_message.into());

                // This sets the state to Handshake state - this prompts the task above to move the state
                // to transport mode so that the next incoming message will be decoded correctly
                // It is important to do this directly before sending the fourth message
                self.set_state(transport_mode).await;
                while !TRANSPORT_READY.load(std::sync::atomic::Ordering::SeqCst) {
                    std::hint::spin_loop()
                }
            }
        };
        let receiver_incoming = self.sender_outgoing.subscribe();
        Ok((receiver_incoming, self.sender_outgoing.clone()))
    }

    async fn handle_connection(&self, mut stream: TcpStream) {
        let mut decoder = StandardNoiseDecoder::<Message>::new();
        let (mut reader, mut writer) = stream.split();

        let read = async {
            loop {
                let writable = decoder.writable();
                match reader.read_exact(writable).await {
                    Ok(_) => {
                        let mut mutable_state = self.state.write().await;
                        let decoded = decoder.next_frame(&mut mutable_state);
                        match decoded {
                            Ok(x) => {
                                if self.sender_incoming.send(x).is_err() {
                                    error!("Shutting down noise stream reader!");
                                    task::yield_now().await;
                                    break;
                                }
                            }
                            Err(e) => {
                                if let codec_sv2::Error::MissingBytes(_) = e {
                                } else {
                                    error!("Shutting down noise stream reader! {:#?}", e);
                                    // sender_incoming.close();
                                    task::yield_now().await;
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        dbg!(&e);
                        //kill thread without a panic - don't need to panic everytime a client
                        // disconnects
                        // sender_incoming.close();
                        task::yield_now().await;
                        break;
                    }
                }
            }
        };
        let write = async {
            let mut encoder = codec_sv2::NoiseEncoder::<Message>::new();
            let mut receiver_outgoing = self.sender_outgoing.subscribe();
            loop {
                let received = receiver_outgoing.recv().await;
                match received {
                    Ok(frame) => {
                        let mut mutable_state = self.state.write().await;
                        let b = encoder.encode(frame, &mut mutable_state).unwrap();
                        let b = b.as_ref();
                        match (writer).write_all(b).await {
                            Ok(_) => (),
                            Err(e) => {
                                dbg!(&e);
                                let _ = writer.shutdown().await;
                                // Just fail and force to reinitialize everything
                                task::yield_now().await;
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        dbg!(&e);
                        // Just fail and force to reinitialize everything
                        let _ = writer.shutdown().await;
                        task::yield_now().await;
                        break;
                    }
                };
                HANDSHAKE_READY.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        };

        tokio::select!(
          _ = read => {},
          _ = write => {},
        );
    }

    async fn set_state(&self, state: codec_sv2::State) {
        loop {
            if HANDSHAKE_READY.load(std::sync::atomic::Ordering::SeqCst) {
                let mut connection = self.state.write().await;
                *connection = state;
                TRANSPORT_READY.store(true, std::sync::atomic::Ordering::Relaxed);
                break;
            }
            task::yield_now().await;
        }
    }
}

static HANDSHAKE_READY: AtomicBool = AtomicBool::new(false);
static TRANSPORT_READY: AtomicBool = AtomicBool::new(false);
