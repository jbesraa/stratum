use std::convert::TryInto;

use const_sv2::MESSAGE_TYPE_OPEN_EXTENDED_MINING_CHANNEL_SUCCES;
use integration_tests_sv2::*;
use roles_logic_sv2::{mining_sv2::OpenExtendedMiningChannelSuccess, parsers::{AnyMessage, Mining}};
use sniffer::{MessageDirection, ReplaceMessage};


#[tokio::test]
async fn tproxy_refuses_bad_extranonce_size() {
    start_tracing();

    let (_tp, tp_addr) = start_template_provider(None);
    let (_pool, pool_addr) = start_pool(Some(tp_addr)).await;

    let message_replacement = AnyMessage::Mining(Mining::OpenExtendedMiningChannelSuccess(
        OpenExtendedMiningChannelSuccess {
            request_id: 0,
            channel_id: 1,
            target: [
                112, 123, 89, 188, 221, 164, 162, 167, 139, 39, 104, 137, 2, 111, 185, 17, 165, 85,
                33, 115, 67, 45, 129, 197, 134, 103, 128, 151, 59, 19, 0, 0,
            ]
            .into(),
            extranonce_prefix: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
                .try_into()
                .unwrap(),
            extranonce_size: 44, // bad extranonce size
        },
    ));
    let replace_message = ReplaceMessage::new(
        MessageDirection::ToDownstream,
        MESSAGE_TYPE_OPEN_EXTENDED_MINING_CHANNEL_SUCCES,
        message_replacement,
    );

    // this sniffer will replace OpenExtendedMiningChannelSuccess with a bad extranonce size
    let (sniffer, sniffer_addr) =
        start_sniffer("0".to_string(), pool_addr, false, Some(replace_message.into()));

    let (_, tproxy_addr) = start_sv2_translator(sniffer_addr).await;

    // make sure tProxy shut down (expected behavior)
    // we only assert that the listening port is now available
    assert!(tokio::net::TcpListener::bind(tproxy_addr).await.is_ok());
}
