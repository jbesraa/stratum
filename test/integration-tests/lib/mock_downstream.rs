use crate::{
    message_aggregator::MessagesAggregator,
    types::{MessageFrame, MsgType},
    utils::{create_downstream, create_upstream, message_from_frame, wait_for_client},
};
use async_channel::Sender;
use codec_sv2::{StandardEitherFrame, Sv2Frame};
use roles_logic_sv2::parsers::AnyMessage;
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub struct MockDownstream {
    upstream_address: SocketAddr,
    messages_from_upstream: MessagesAggregator,
}

impl MockDownstream {
    pub fn new(upstream_address: SocketAddr) -> Self {
        Self {
            upstream_address,
            messages_from_upstream: MessagesAggregator::new(),
        }
    }

    pub async fn start(&self) -> Sender<MessageFrame> {
        let upstream_address = self.upstream_address;
        let (upstream_receiver, upstream_sender) = create_upstream(
            TcpStream::connect(upstream_address)
                .await
                .expect("Failed to connect to upstream"),
        )
        .await
        .expect("Failed to create upstream");
        let messages_from_upstream = self.messages_from_upstream.clone();
        tokio::spawn(async move {
            while let Ok(mut frame) = upstream_receiver.recv().await {
                let (msg_type, msg) = message_from_frame(&mut frame);
                messages_from_upstream.add_message(msg_type, msg);
            }
        });
        upstream_sender
    }

    pub fn next_message_from_upstream(&self) -> Option<(MsgType, AnyMessage<'static>)> {
        self.messages_from_upstream.next_message()
    }
}

pub struct MockUpstream {
    listening_address: SocketAddr,
    messages_from_dowsntream: MessagesAggregator,
    // This holds the list of tuples. Each tuple refers to message received and what response
    // should the upstream send back.
    response_messages: Vec<(MsgType, AnyMessage<'static>)>,
}

impl MockUpstream {
    pub fn new(
        listening_address: SocketAddr,
        response_messages: Vec<(MsgType, AnyMessage<'static>)>,
    ) -> Self {
        Self {
            listening_address,
            messages_from_dowsntream: MessagesAggregator::new(),
            response_messages,
        }
    }

    pub async fn start(&self) -> Sender<MessageFrame> {
        let listening_address = self.listening_address;
        let (upstream_receiver, upstream_sender) = // FIX NAMING
            create_downstream(wait_for_client(listening_address).await)
                .await
                .expect("Failed to create upstream");
        let messages_from_dowsntream = self.messages_from_dowsntream.clone();
        let response_messages = self.response_messages.clone();
        let sender = upstream_sender.clone();
        tokio::spawn(async move {
            while let Ok(mut frame) = upstream_receiver.recv().await {
                let (msg_type, msg) = message_from_frame(&mut frame);
                messages_from_dowsntream.add_message(msg_type, msg);
                // loop through the response messages and send them back to the client
                let response = response_messages
                    .iter()
                    .find(|(msg_type, _)| msg_type == msg_type);
                if let Some((_, response_msg)) = response {
                    let message = StandardEitherFrame::<AnyMessage<'_>>::Sv2(
                        Sv2Frame::from_message(response_msg.clone(), msg_type, 0, false)
                            .expect("Failed to create the frame"),
                    );
                    sender.send(message).await.unwrap();
                }
            }
        });
        upstream_sender
    }

    pub fn next_message_from_downstream(&self) -> Option<(MsgType, AnyMessage<'static>)> {
        self.messages_from_dowsntream.next_message()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::start_template_provider;
    use codec_sv2::{StandardEitherFrame, Sv2Frame};
    use roles_logic_sv2::{
        common_messages_sv2::{Protocol, SetupConnection},
        parsers::CommonMessages,
    };
    use std::convert::TryInto;
    use stratum_common::{MESSAGE_TYPE_SETUP_CONNECTION, MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS};

    #[tokio::test]
    async fn test_mock_downstream() {
        let (_tp, socket) = start_template_provider(None);
        let mock_downstream = MockDownstream::new(socket);
        let send_to_upstream = mock_downstream.start().await;
        let setup_connection =
            AnyMessage::Common(CommonMessages::SetupConnection(SetupConnection {
                protocol: Protocol::TemplateDistributionProtocol,
                min_version: 2,
                max_version: 2,
                flags: 0,
                endpoint_host: b"0.0.0.0".to_vec().try_into().unwrap(),
                endpoint_port: 8081,
                vendor: b"Bitmain".to_vec().try_into().unwrap(),
                hardware_version: b"901".to_vec().try_into().unwrap(),
                firmware: b"abcX".to_vec().try_into().unwrap(),
                device_id: b"89567".to_vec().try_into().unwrap(),
            }));
        let message = StandardEitherFrame::<AnyMessage<'_>>::Sv2(
            Sv2Frame::from_message(setup_connection, MESSAGE_TYPE_SETUP_CONNECTION, 0, false)
                .expect("Failed to create the frame"),
        );
        send_to_upstream.send(message).await.unwrap();
        mock_downstream
            .messages_from_upstream
            .has_message_type(MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS);
    }

    #[tokio::test]
    async fn test_upstream_mock() {}
}
