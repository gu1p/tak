pub(in crate::daemon::protocol::broker) trait BrokerRemoteIo:
    tokio::io::AsyncRead + tokio::io::AsyncWrite
{
}
impl<T> BrokerRemoteIo for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite + ?Sized {}
pub(in crate::daemon::protocol::broker) type BrokerRemoteStream =
    Box<dyn BrokerRemoteIo + Unpin + Send>;

#[derive(Debug, Clone)]
pub struct BrokerForwardResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub(in crate::daemon::protocol) struct BrokerRemoteHttpRequest<'a> {
    pub endpoint: &'a str,
    pub node_id: &'a str,
    pub bearer_token: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub headers: &'a [(String, String)],
    pub body: &'a [u8],
}
