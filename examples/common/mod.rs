use ansi_term::Color::{Green, Purple, Red, Yellow};
use boomnet::service::endpoint::Context;
use boomnet::service::endpoint::ws::{TlsWebsocket, TlsWebsocketEndpoint, TlsWebsocketEndpointWithContext};
use boomnet::stream::mio::{IntoMioStream, MioStream};
use boomnet::stream::tcp::TcpStream;
use boomnet::stream::tls::{IntoTlsStream, TlsConfigExt};
use boomnet::stream::{ConnectionInfo, ConnectionInfoProvider};
use boomnet::ws::{IntoTlsWebsocket, IntoWebsocket, WebsocketFrame};
use log::info;
use std::io;
use std::net::SocketAddr;

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
    subscribe: bool,
}

impl TradeEndpoint {
    #[allow(dead_code)]
    pub fn new(id: u32, url: &'static str, net_iface: Option<&'static str>, instrument: &'static str) -> TradeEndpoint {
        Self::new_with_subscribe(id, url, net_iface, instrument, true)
    }

    pub fn new_with_subscribe(
        id: u32,
        url: &'static str,
        net_iface: Option<&'static str>,
        instrument: &'static str,
        subscribe: bool,
    ) -> TradeEndpoint {
        let (mut connection_info, ws_endpoint, _) = boomnet::ws::util::parse_url(url).unwrap();
        if let Some(net_iface) = net_iface {
            connection_info = connection_info.with_net_iface_from_name(net_iface);
        }
        Self {
            id,
            connection_info,
            instrument,
            ws_endpoint,
            subscribe,
        }
    }

    pub fn subscribe(&mut self, ws: &mut TlsWebsocket<MioStream>) -> io::Result<()> {
        ws.send_text(
            true,
            Some(format!(r#"{{"method":"SUBSCRIBE","params":["{}@trade"],"id":1}}"#, self.instrument).as_bytes()),
        )?;
        Ok(())
    }
}

impl ConnectionInfoProvider for TradeEndpoint {
    fn connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }
}

impl TlsWebsocketEndpoint for TradeEndpoint {
    type Stream = MioStream;

    fn create_websocket(&mut self, addr: SocketAddr) -> io::Result<Option<TlsWebsocket<Self::Stream>>> {
        let mut ws = TcpStream::try_from((&self.connection_info, addr))?
            .into_mio_stream()
            .into_tls_stream_with_config(|cfg| cfg.with_no_cert_verification())?
            .into_websocket(&self.ws_endpoint);

        if self.subscribe {
            self.subscribe(&mut ws)?;
        }

        Ok(Some(ws))
    }

    #[inline]
    fn poll(&mut self, ws: &mut TlsWebsocket<Self::Stream>) -> io::Result<()> {
        for frame in ws.read_batch()? {
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

    fn create_websocket(
        &mut self,
        addr: SocketAddr,
        _ctx: &mut FeedContext,
    ) -> io::Result<Option<TlsWebsocket<Self::Stream>>> {
        let mut ws = TcpStream::try_from((&self.connection_info, addr))?
            .into_mio_stream()
            .into_tls_websocket(&self.ws_endpoint)?;

        if self.subscribe {
            self.subscribe(&mut ws)?;
        }

        Ok(Some(ws))
    }

    #[inline]
    fn poll(&mut self, ws: &mut TlsWebsocket<Self::Stream>, _ctx: &mut FeedContext) -> io::Result<()> {
        for frame in ws.read_batch()? {
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
