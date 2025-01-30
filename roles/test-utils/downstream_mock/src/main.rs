use codec_sv2::StandardEitherFrame;
use connection::SV2ConnectionSetup;
use key_utils::{Secp256k1PublicKey, Secp256k1SecretKey};
use roles_logic_sv2::parsers::AnyMessage;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast::{Receiver, Sender},
};

async fn create_downstream(
    upstream_address: String,
) -> Option<(
    Receiver<StandardEitherFrame<AnyMessage<'static>>>,
    Sender<StandardEitherFrame<AnyMessage<'static>>>,
)> {
    let stream = TcpStream::connect(upstream_address)
        .await
        .expect("Failed to connect to upstream");
    let connection = SV2ConnectionSetup::new(connection::ConnectionArgs::Downstream(None));
    connection.start(stream).await;
    None
    // if let Ok((receiver_from_server, sender_to_server)) = connection.start(stream).await {
    //     Some((receiver_from_server, sender_to_server))
    // } else {
    //     None
    // }
}

async fn create_upstream(
    listening_address: String,
) -> Option<(
    Receiver<StandardEitherFrame<AnyMessage<'static>>>,
    Sender<StandardEitherFrame<AnyMessage<'static>>>,
)> {
    let listener = TcpListener::bind(listening_address).await.unwrap();
    while let Ok((stream, _)) = listener.accept().await {
        let pub_key = "9auqWEzQDVyd2oe1JVGFLMLHZtCo2FFqZwtKA5gd9xbuEu7PH72"
            .to_string()
            .parse::<Secp256k1PublicKey>()
            .unwrap();
        let prv_key = "mkDLTBBRxdBv998612qipDYoTK3YUrqLe8uWw7gu3iXbSrn2n"
            .to_string()
            .parse::<Secp256k1SecretKey>()
            .unwrap();
        let connection =
            SV2ConnectionSetup::new(connection::ConnectionArgs::Upstream(pub_key, prv_key));
        let ret = connection.start(stream).await;
        dbg!(&ret);
    }
    None
}

#[tokio::main]
async fn main() {
    let upstream_address = "127.0.0.1:12345".to_string();
    tokio::join!(
        create_upstream(upstream_address.clone()),
        create_downstream(upstream_address.clone()),
    );
    println!("Hello, world!");
}
