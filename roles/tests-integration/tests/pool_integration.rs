mod common;

use std::str::FromStr;

use roles_logic_sv2::{
    common_messages_sv2::{Protocol, SetupConnection},
    parsers::{CommonMessages, PoolMessages, TemplateDistribution},
};

// This test starts a Template Provider and a Pool, and checks if they exchange the correct
// messages upon connection.
// The Sniffer is used as a proxy between the Upstream(Template Provider) and Downstream(Pool). The
// Pool will connect to the Sniffer, and the Sniffer will connect to the Template Provider.
#[tokio::test]
async fn success_pool_template_provider_connection() {
    let sniffer_addr = common::get_available_address();
    let tp_addr = common::get_available_address();
    let pool_addr = common::get_available_address();
    let _tp = common::start_template_provider(tp_addr.port()).await;
    let sniffer = common::start_sniffer(sniffer_addr, tp_addr).await;
    let _ = common::start_pool(Some(pool_addr), Some(sniffer_addr)).await;
    // here we assert that the downstream(pool in this case) have sent `SetupConnection` message
    // with the correct parameters, protocol, flags, min_version and max_version.  Note that the
    // macro can take any number of arguments after the message argument, but the order is
    // important where a property should be followed by its value.
    assert_common_message!(
        &sniffer.next_downstream_message(),
        SetupConnection,
        protocol,
        Protocol::TemplateDistributionProtocol,
        flags,
        0,
        min_version,
        2,
        max_version,
        2
    );
    assert_common_message!(&sniffer.next_upstream_message(), SetupConnectionSuccess);
    assert_tp_message!(&sniffer.next_downstream_message(), CoinbaseOutputDataSize);
    assert_tp_message!(&sniffer.next_upstream_message(), NewTemplate);
    assert_tp_message!(sniffer.next_upstream_message(), SetNewPrevHash);
}

// "This test does",
// "Launch a pool mocks",
// "Launch jds",
// "Launch jdc",
// "Launch mining-proxy",
// "Launch a mining device",
// "Act like a pool",
// "Connect jdc to the first pool",
// "Receive an submit share error from first pool",
// "Close jdc connection with first pool",
// "Connect jdc the pool mocked by this test"
#[tokio::test]
#[ignore]
async fn test_jdc_fallback_after_submit_share_error() {
    let tp_addr = common::get_available_address();
    let pool_addr = common::get_available_address();
    let _tp = common::start_template_provider(tp_addr.port()).await;
    let _ = common::start_pool(Some(pool_addr), Some(tp_addr)).await;
    let jds_addr = common::start_jds(tp_addr).await;
    common::start_jdc(
        pool_addr,
        tp_addr,
        std::net::SocketAddr::from_str(&jds_addr).unwrap(),
    )
    .await;
}
