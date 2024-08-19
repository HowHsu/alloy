use alloy_json_rpc::{Response, ResponsePayload, SerializedRequest};
use alloy_primitives::U256;
use alloy_transport::TransportResult;
use std::fmt;
use tokio::sync::oneshot;

/// An in-flight JSON-RPC request.
///
/// This struct contains the request that was sent, as well as a channel to
/// receive the response on.
pub(crate) struct InFlight {
    /// The request
    pub(crate) request: SerializedRequest,

    /// The number of items to buffer in the subscription channel.
    pub(crate) channel_size: usize,

    /// The channel to send the response on.
    pub(crate) tx: oneshot::Sender<TransportResult<Response>>,
}

impl fmt::Debug for InFlight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InFlight")
            .field("request", &self.request)
            .field("channel_size", &self.channel_size)
            .field("tx_is_closed", &self.tx.is_closed())
            .finish()
    }
}

impl InFlight {
    /// Create a new in-flight request.
    pub(crate) fn new(
        request: SerializedRequest,
        channel_size: usize,
    ) -> (Self, oneshot::Receiver<TransportResult<Response>>) {
        let (tx, rx) = oneshot::channel();

        (Self { request, channel_size, tx }, rx)
    }

    /// Check if the request is a subscription.
    pub(crate) fn is_subscription(&self) -> bool {
        self.request.is_subscription()
    }

    /// Get a reference to the serialized request.
    ///
    /// This is used to (re-)send the request over the transport.
    pub(crate) const fn request(&self) -> &SerializedRequest {
        &self.request
    }

    fn char_to_uint(c: char) -> u32 {
        match c {
            '0'..='9' => c as u32 - '0' as u32,
            'A'..='Z' => c as u32 - 'A' as u32 + 10,
            'a'..='z' => c as u32 - 'a' as u32 + 36,
            _ => panic!("invalid subscription id char: {}", c),
        }
    }
    /// Fulfill the request with a response. This consumes the in-flight
    /// request. If the request is a subscription and the response is not an
    /// error, the subscription ID and the in-flight request are returned.
    pub(crate) fn fulfill(self, resp: Response) -> Option<(U256, Self)> {
        if self.is_subscription() {
            if let ResponsePayload::Success(val) = resp.payload {
                let mut id = U256::ZERO;
                let raw_val = val.get().trim_matches('"');

                for ch in raw_val.chars() {
                    id = id * U256::from(100) + U256::from(Self::char_to_uint(ch));
                }
                return Some((id, self));
            }
        }

        let _ = self.tx.send(Ok(resp));
        None
    }
}
