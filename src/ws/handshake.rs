use std::collections::VecDeque;
use std::io;
use std::io::ErrorKind::WouldBlock;
use std::io::{Read, Write};

use base64::Engine;
use base64::engine::general_purpose;
use http::StatusCode;
use httparse::Response;
use rand::{Rng, rng};

use crate::buffer::ReadBuffer;
use crate::ws::Error;
use crate::ws::handshake::HandshakeState::{Completed, NotStarted, Pending};

#[derive(Debug)]
pub struct Handshaker {
    buffer: ReadBuffer<1>,
    state: HandshakeState,
    server_name: String,
    endpoint: String,
    pending_msg_buffer: VecDeque<(u8, bool, Option<Vec<u8>>)>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HandshakeState {
    NotStarted,
    Pending,
    Completed,
}

impl Handshaker {
    pub fn new(server_name: &str, endpoint: &str) -> Self {
        Self {
            buffer: ReadBuffer::new(),
            state: NotStarted,
            server_name: server_name.to_string(),
            endpoint: endpoint.to_string(),
            pending_msg_buffer: VecDeque::with_capacity(256),
        }
    }

    #[cold]
    pub fn read<S: Read>(&mut self, stream: &mut S) -> io::Result<()> {
        if self.state == Pending {
            self.buffer.read_from(stream)?;
        }
        Ok(())
    }

    #[cold]
    pub fn perform_handshake<S: Read + Write>(&mut self, stream: &mut S) -> io::Result<()> {
        match self.state {
            NotStarted => {
                self.send_handshake_request(stream)?;
                Err(io::Error::from(WouldBlock))
            }
            Pending => {
                let available = self.buffer.available();
                if available >= 4 && self.buffer.view_last(4) == b"\r\n\r\n" {
                    // decode http response
                    let mut headers = [httparse::EMPTY_HEADER; 64];
                    let mut response = Response::new(&mut headers);
                    response.parse(self.buffer.view()).map_err(io::Error::other)?;
                    if response.code.unwrap() != StatusCode::SWITCHING_PROTOCOLS.as_u16() {
                        return Err(io::Error::other("unable to switch protocols"));
                    }
                    self.state = Completed;
                }
                Err(io::Error::from(WouldBlock))
            }
            Completed => Ok(()),
        }
    }

    #[cold]
    pub fn buffer_message(&mut self, fin: bool, op: u8, body: Option<&[u8]>) {
        let body = body.map(|body| body.to_vec());
        self.pending_msg_buffer.push_back((op, fin, body))
    }

    #[cold]
    pub fn drain_pending_message_buffer<S, F>(&mut self, stream: &mut S, mut send: F) -> Result<(), Error>
    where
        S: Write,
        F: FnMut(&mut S, bool, u8, Option<&[u8]>) -> io::Result<()>,
    {
        while let Some((op, fin, body)) = self.pending_msg_buffer.pop_front() {
            send(stream, fin, op, body.as_deref())?;
        }
        Ok(())
    }

    fn send_handshake_request<S: Write>(&mut self, stream: &mut S) -> io::Result<()> {
        stream.write_all(format!("GET {} HTTP/1.1\r\n", self.endpoint).as_bytes())?;
        stream.write_all(format!("Host: {}\r\n", self.server_name).as_bytes())?;
        stream.write_all(b"Upgrade: websocket\r\n")?;
        stream.write_all(b"Connection: upgrade\r\n")?;
        stream.write_all(format!("Sec-WebSocket-Key: {}\r\n", generate_nonce()).as_bytes())?;
        stream.write_all(b"Sec-WebSocket-Version: 13\r\n")?;
        stream.write_all(b"\r\n")?;
        stream.flush()?;
        self.state = Pending;
        Ok(())
    }
}

fn generate_nonce() -> String {
    let mut rng = rng();
    let nonce_bytes: [u8; 16] = rng.random();
    general_purpose::STANDARD.encode(nonce_bytes)
}
