// #![allow(dead_code)]
// use const_sv2::{
//     INITIATOR_EXPECTED_HANDSHAKE_MESSAGE_SIZE, RESPONDER_EXPECTED_HANDSHAKE_MESSAGE_SIZE,
// };
// use key_utils::{Secp256k1PublicKey, Secp256k1SecretKey};
// use roles_logic_sv2::parsers::AnyMessage;
// use std::{
//     convert::TryInto,
//     sync::{atomic::AtomicBool, Arc},
// };
// use tokio::{
//     io::{AsyncReadExt, AsyncWriteExt},
//     net::TcpStream,
//     sync::{
//         broadcast::{channel, Receiver, Sender},
//         RwLock,
//     },
//     task::{self},
// };

// use codec_sv2::{
//     HandShakeFrame, HandshakeRole, Initiator, Responder, StandardEitherFrame, StandardNoiseDecoder,
// };

// use tracing::error;

// // #[derive(Debug, Clone)]
// pub struct SV2ConnectionSetup {
//     role: Arc<RwLock<HandshakeRole>>,
//     state: Arc<RwLock<codec_sv2::State>>,
//     sender_incoming: Sender<StandardEitherFrame<AnyMessage<'static>>>,
//     sender_outgoing: Sender<StandardEitherFrame<AnyMessage<'static>>>,
// }

// // #[derive(Debug, Clone)]
// pub enum RoleConnectionSetup {
//     Downstream(DownstreamConnectionSetup),
//     Upstream(UpstreamConnectionSetup),
// }

// impl RoleConnectionSetup {
//     pub async fn start(self, stream: TcpStream) {
//         match self {
//             RoleConnectionSetup::Downstream(downstream) => downstream.start(stream).await,
//             RoleConnectionSetup::Upstream(upstream) => upstream.start(stream).await,
//         }
//     }
// }

// impl DownstreamConnectionSetup {
//     pub async fn start(mut self) {
//         let mut decoder = StandardNoiseDecoder::<AnyMessage<'static>>::new();

//         // Create and send first handshake message
//         let role = if let Some(_pub_key) = self.pub_key {
//             todo!();
//         } else {
//             HandshakeRole::Initiator(Initiator::without_pk().unwrap())
//         };
//         let mut write_state = self.state.write().await;
//         *write_state = codec_sv2::State::initialized(role);
//         dbg!("after set state");
//         // let mut receiver_incoming = self.sender_incoming.subscribe();
//         let first_message = write_state.step_0().unwrap();
//         // dbg!(&first_message);
//         dbg!("after first message from downstream");
//         let _ = match self.sender_outgoing.send(first_message.into()) {
//             Ok(a) => {
//                 dbg!("downstream sent first message {}", a);
//             }
//             Err(e) => {
//                 dbg!(&e);
//                 return;
//             }
//         };
//         dbg!("downstream sent first message");
//         // Receive and deserialize second handshake message
//         let second_message = self.receiver_incoming.recv().await.unwrap();
//         dbg!(&second_message);
//         let second_message: HandShakeFrame = second_message.try_into().unwrap();
//         let second_message = second_message.get_payload_when_handshaking();
//         dbg!("second message: {:?}", second_message.len());
//         dbg!(&INITIATOR_EXPECTED_HANDSHAKE_MESSAGE_SIZE);
//         let second_message: [u8; INITIATOR_EXPECTED_HANDSHAKE_MESSAGE_SIZE] =
//             second_message.try_into().unwrap();
//         // Create and send thirth handshake message
//         let transport_mode = write_state.step_2(second_message).unwrap();
//         *write_state = transport_mode;

//         // let mut state = codec_sv2::State::initialized(role);

//         // // Create and send first handshake message
//         // let first_message = state.step_0()?;
//         // sender_outgoing.send(first_message.into()).await?;

//         // // Receive and deserialize second handshake message
//         // let second_message = receiver_incoming.recv().await?;
//         // let second_message: HandShakeFrame = second_message
//         //     .try_into()
//         //     .map_err(|_| Error::HandshakeRemoteInvalidMessage)?;
//         // let second_message: [u8; INITIATOR_EXPECTED_HANDSHAKE_MESSAGE_SIZE] = second_message
//         //     .get_payload_when_handshaking()
//         //     .try_into()
//         //     .map_err(|_| Error::HandshakeRemoteInvalidMessage)?;

//         // // Create and send thirth handshake message
//         // let transport_mode = state.step_2(second_message)?;

//         // T::set_state(self_, transport_mode).await;
//         // while !TRANSPORT_READY.load(std::sync::atomic::Ordering::SeqCst) {
//         //     std::hint::spin_loop()
//         // }
//         // Ok(())
//     }
//     // sender_outgoing: Sender<StandardEitherFrame<Message>>,
//     // receiver_incoming: Receiver<StandardEitherFrame<Message>>,

//     async fn handshake(&mut self) {
//         // Create and send first handshake message
//         let role = if let Some(_pub_key) = self.pub_key {
//             todo!();
//         } else {
//             HandshakeRole::Initiator(Initiator::without_pk().unwrap())
//         };
//         let mut write_state = self.state.write().await;
//         *write_state = codec_sv2::State::initialized(role);
//         dbg!("after set state");
//         // let mut receiver_incoming = self.sender_incoming.subscribe();
//         let first_message = write_state.step_0().unwrap();
//         // dbg!(&first_message);
//         dbg!("after first message from downstream");
//         let _ = match self.sender_outgoing.send(first_message.into()) {
//             Ok(a) => {
//                 dbg!("downstream sent first message {}", a);
//             }
//             Err(e) => {
//                 dbg!(&e);
//                 return;
//             }
//         };
//         dbg!("downstream sent first message");
//         // Receive and deserialize second handshake message
//         let second_message = self.receiver_incoming.recv().await.unwrap();
//         dbg!(&second_message);
//         let second_message: HandShakeFrame = second_message.try_into().unwrap();
//         let second_message = second_message.get_payload_when_handshaking();
//         dbg!("second message: {:?}", second_message.len());
//         dbg!(&INITIATOR_EXPECTED_HANDSHAKE_MESSAGE_SIZE);
//         let second_message: [u8; INITIATOR_EXPECTED_HANDSHAKE_MESSAGE_SIZE] =
//             second_message.try_into().unwrap();
//         // Create and send thirth handshake message
//         let transport_mode = write_state.step_2(second_message).unwrap();
//         *write_state = transport_mode;
//     }

//     async fn handle_connection(&self, mut stream: TcpStream) {
//         let mut decoder = StandardNoiseDecoder::<AnyMessage<'static>>::new();
//         let (mut reader, mut writer) = stream.split();
//         let read = async {
//             loop {
//                 let writable = decoder.writable();
//                 match reader.read_exact(writable).await {
//                     Ok(_) => {
//                         let mut mutable_state = self.state.write().await;
//                         let decoded = decoder.next_frame(&mut mutable_state);
//                         match decoded {
//                             Ok(x) => {
//                                 if self.sender_incoming.send(x).is_err() {
//                                     break;
//                                 }
//                             }
//                             Err(e) => {
//                                 if let codec_sv2::Error::MissingBytes(_) = e {
//                                 } else {
//                                     error!("Shutting down noise stream reader! {:#?}", e);
//                                 }
//                             }
//                         }
//                     }
//                     Err(e) => {
//                         dbg!(&e);
//                         break;
//                     }
//                 }
//             }
//         };
//         let write = async {
//             let mut encoder = codec_sv2::NoiseEncoder::<AnyMessage<'static>>::new();
//             // let mut receiver_outgoing = self.sender_outgoing.subscribe();
//             let mut reco = self.receiver_outgoing.resubscribe();
//             loop {
//                 let received = reco.recv().await;
//                 match received {
//                     Ok(frame) => {
//                         let mut mutable_state = self.state.write().await;
//                         let b = encoder.encode(frame, &mut mutable_state).unwrap();
//                         let b = b.as_ref();
//                         match (writer).write_all(b).await {
//                             Ok(_) => (),
//                             Err(e) => {
//                                 dbg!(&e);
//                                 let _ = writer.shutdown().await;
//                                 // Just fail and force to reinitialize everything
//                                 task::yield_now().await;
//                                 break;
//                             }
//                         }
//                     }
//                     Err(e) => {
//                         dbg!(&e);
//                         // Just fail and force to reinitialize everything
//                         let _ = writer.shutdown().await;
//                         task::yield_now().await;
//                         break;
//                     }
//                 };
//                 HANDSHAKE_READY.store(true, std::sync::atomic::Ordering::Relaxed);
//             }
//         };

//         tokio::select!(
//           _ = read => {},
//           _ = write => {},
//         );
//     }
// }

// impl UpstreamConnectionSetup {
//     pub async fn start(mut self, stream: TcpStream) {
//         let _ = self.handshake().await;
//     }

//     async fn handshake(&mut self) {
//         dbg!("in upstream handshake");

//         let role = Responder::from_authority_kp(
//             &self.pub_key.into_bytes(),
//             &self.priv_key.into_bytes(),
//             std::time::Duration::from_secs(300000),
//         )
//         .unwrap();

//         let mut write_state = self.state.write().await;
//         *write_state = codec_sv2::State::initialized(HandshakeRole::Responder(role));
//         let mut receiver_outgoing = self.sender_outgoing.subscribe();
//         dbg!("before upstream receives first message");
//         let first_message: HandShakeFrame =
//             receiver_outgoing.recv().await.unwrap().try_into().unwrap();
//         dbg!("after upstream receives first message");
//         let first_message: [u8; RESPONDER_EXPECTED_HANDSHAKE_MESSAGE_SIZE] = first_message
//             .get_payload_when_handshaking()
//             .try_into()
//             .unwrap();

//         dbg!(&first_message);
//         // Create and send second handshake message
//         let (second_message, transport_mode) = write_state.step_1(first_message).unwrap();
//         dbg!(&second_message);
//         HANDSHAKE_READY.store(false, std::sync::atomic::Ordering::SeqCst);
//         let _ = self.sender_outgoing.send(second_message.into());

//         // This sets the state to Handshake state - this prompts the task above to move the state
//         // to transport mode so that the next incoming message will be decoded correctly
//         // It is important to do this directly before sending the fourth message
//         *write_state = transport_mode;
//         // while !TRANSPORT_READY.load(std::sync::atomic::Ordering::SeqCst) {
//         //     std::hint::spin_loop()
//         // }
//     }
// }

// #[derive(Debug, Clone)]
// pub struct DownstreamConnectionSetup {
//     pub_key: Option<Secp256k1PublicKey>,
//     state: Arc<RwLock<codec_sv2::State>>,
//     sender_incoming: Sender<StandardEitherFrame<AnyMessage<'static>>>,
//     sender_outgoing: Sender<StandardEitherFrame<AnyMessage<'static>>>,
// }

// #[derive(Debug, Clone)]
// pub struct UpstreamConnectionSetup {
//     pub_key: Secp256k1PublicKey,
//     priv_key: Secp256k1SecretKey,
//     state: Arc<RwLock<codec_sv2::State>>,
//     sender_incoming: Sender<StandardEitherFrame<AnyMessage<'static>>>,
//     sender_outgoing: Sender<StandardEitherFrame<AnyMessage<'static>>>,
// }

// pub enum ConnectionArgs {
//     Downstream(Option<Secp256k1PublicKey>),
//     Upstream(Secp256k1PublicKey, Secp256k1SecretKey),
// }

// impl SV2ConnectionSetup {
//     pub fn new(connection_data: ConnectionArgs) -> RoleConnectionSetup {
//         let role = match connection_data {
//             ConnectionArgs::Downstream(pub_key) => {
//                 if let Some(_pub_key) = pub_key {
//                     todo!();
//                 } else {
//                     HandshakeRole::Initiator(Initiator::without_pk().unwrap())
//                 }
//             }
//             ConnectionArgs::Upstream(pub_key, priv_key) => HandshakeRole::Responder(
//                 Responder::from_authority_kp(
//                     &pub_key.into_bytes(),
//                     &priv_key.into_bytes(),
//                     std::time::Duration::from_secs(300000),
//                 )
//                 .unwrap(),
//             ),
//         };
//         let state = Arc::new(RwLock::new(codec_sv2::State::not_initialized(&role)));
//         let (sender_incoming, receiver_incoming): (
//             Sender<StandardEitherFrame<AnyMessage<'static>>>,
//             Receiver<StandardEitherFrame<AnyMessage<'static>>>,
//         ) = channel(10);
//         let (sender_outgoing, receiver_outgoing): (
//             Sender<StandardEitherFrame<AnyMessage<'static>>>,
//             Receiver<StandardEitherFrame<AnyMessage<'static>>>,
//         ) = channel(10);
//         // let sub = sender_outgoing.subscribe();
//         match connection_data {
//             ConnectionArgs::Downstream(pub_key) => {
//                 RoleConnectionSetup::Downstream(DownstreamConnectionSetup {
//                     pub_key,
//                     state,
//                     sender_incoming,
//                     sender_outgoing,
//                 })
//             }
//             ConnectionArgs::Upstream(pub_key, priv_key) => {
//                 RoleConnectionSetup::Upstream(UpstreamConnectionSetup {
//                     pub_key,
//                     priv_key,
//                     state,
//                     sender_incoming,
//                     sender_outgoing,
//                 })
//             }
//         }
//     }
// }

// static HANDSHAKE_READY: AtomicBool = AtomicBool::new(false);
// static TRANSPORT_READY: AtomicBool = AtomicBool::new(false);
