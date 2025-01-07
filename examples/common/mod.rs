use ansi_term::Color::{Green, Purple, Red, Yellow};
use boomnet::inet::{IntoNetworkInterface, ToSocketAddr};
use boomnet::service::endpoint::ws::{TlsWebsocket, TlsWebsocketEndpoint, TlsWebsocketEndpointWithContext};
use boomnet::service::endpoint::Context;
use boomnet::stream::mio::{IntoMioStream, MioStream};
use boomnet::stream::{ConnectionInfo, ConnectionInfoProvider};
use boomnet::ws::{IntoTlsWebsocket, WebsocketFrame};
use log::info;
use std::io;
use std::net::SocketAddr;
use url::Url;

pub struct FeedContext;
impl Context for FeedContext {}

impl FeedContext {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }
}

pub struct TradeEndpoint {
    id: u32,
    connection_info: ConnectionInfo,
    instrument: &'static str,
    ws_endpoint: String,
}

impl TradeEndpoint {
    pub fn new(id: u32, url: &'static str, net_iface: Option<&'static str>, instrument: &'static str) -> TradeEndpoint {
        let url = Url::parse(url).unwrap();
        let mut connection_info = ConnectionInfo::try_from(url.clone()).unwrap();
        let ws_endpoint = url.path().to_owned();
        let net_iface = net_iface
            .and_then(|name| name.into_network_interface())
            .and_then(|iface| iface.to_socket_addr());
        if let Some(net_iface) = net_iface {
            connection_info = connection_info.with_net_iface(net_iface);
        }
        Self {
            id,
            connection_info,
            instrument,
            ws_endpoint,
        }
    }
}

impl ConnectionInfoProvider for TradeEndpoint {
    fn connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }
}

impl TlsWebsocketEndpoint for TradeEndpoint {
    type Stream = MioStream;

    fn create_websocket(&mut self, addr: SocketAddr) -> io::Result<TlsWebsocket<Self::Stream>> {
        let mut ws = self
            .connection_info
            .clone()
            .into_tcp_stream_with_addr(addr)?
            .into_mio_stream()
            .into_tls_websocket(&self.ws_endpoint);

        ws.send_text(
            true,
            Some(format!(r#"{{"method":"SUBSCRIBE","params":["{}@trade"],"id":1}}"#, self.instrument).as_bytes()),
        )?;

        Ok(ws)
    }

    #[inline]
    fn poll(&mut self, ws: &mut TlsWebsocket<Self::Stream>) -> io::Result<()> {
        for frame in ws.batch_iter()? {
            if let WebsocketFrame::Text(fin, data) = frame? {
                match self.id % 4 {
                    0 => info!("({fin}) {}", Red.paint(String::from_utf8_lossy(data))),
                    1 => info!("({fin}) {}", Green.paint(String::from_utf8_lossy(data))),
                    2 => info!("({fin}) {}", Purple.paint(String::from_utf8_lossy(data))),
                    3 => info!("({fin}) {}", Yellow.paint(String::from_utf8_lossy(data))),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

impl TlsWebsocketEndpointWithContext<FeedContext> for TradeEndpoint {
    type Stream = MioStream;

    fn create_websocket(&mut self, addr: SocketAddr, _ctx: &mut FeedContext) -> io::Result<TlsWebsocket<Self::Stream>> {
        let mut ws = self
            .connection_info
            .clone()
            .into_tcp_stream_with_addr(addr)?
            .into_mio_stream()
            .into_tls_websocket(&self.ws_endpoint);

        ws.send_text(
            true,
            Some(format!(r#"{{"method":"SUBSCRIBE","params":["{}@trade"],"id":1}}"#, self.instrument).as_bytes()),
        )?;

        Ok(ws)
    }

    #[inline]
    fn poll(&mut self, ws: &mut TlsWebsocket<Self::Stream>, _ctx: &mut FeedContext) -> io::Result<()> {
        for frame in ws.batch_iter()? {
            if let WebsocketFrame::Text(fin, data) = frame? {
                match self.id % 4 {
                    0 => info!("({fin}) {}", Red.paint(String::from_utf8_lossy(data))),
                    1 => info!("({fin}) {}", Green.paint(String::from_utf8_lossy(data))),
                    2 => info!("({fin}) {}", Purple.paint(String::from_utf8_lossy(data))),
                    3 => info!("({fin}) {}", Yellow.paint(String::from_utf8_lossy(data))),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
