use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write, Result, Error, ErrorKind};

pub struct GdbConnection {
    client: TcpStream,
    buffer: Vec<u8>,
}

impl GdbConnection {
    pub fn new(port: u16) -> Result<Self> {
        let listener = TcpListener::bind(("localhost", port))?;
        let (client, _) = listener.accept()?;

        client.set_nonblocking(true)?;
        client.set_nodelay(true)?;

        Ok(Self {
            client,
            buffer: Vec::new()
        })
    }

    pub fn read_packet(&mut self) -> Result<Option<String>> {
        let mut buf = [0u8; 65535];

        match self.client.read(&mut buf) {
            Ok(0) => {
                return Err(Error::new(ErrorKind::ConnectionAborted, "disconnected"));
            }
            Ok(n) => {
                self.buffer.extend_from_slice(&buf[..n]);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => { }
            Err(e) => { return Err(e); }
        }

        if let Some((start, end)) = Self::find_packet_end(&self.buffer) {
            if end-start == 1 && self.buffer[start] == b'\x03' {
                self.buffer.drain(..end);
                return Ok(Some(String::from("\x03")));
            }

            let s = String::from_utf8_lossy(&self.buffer[start..end]).to_string();
            self.buffer.drain(..end);

            if let Some(content_end) = s.find('#') {
                let content = String::from(&s[1..content_end]);
                // println!("gdb -> {:?}", content);
                return Ok(Some(content));
            }
        }

        Ok(None)
    }

    fn find_packet_end(buf: &[u8]) -> Option<(usize, usize)> {
        let mut start = None;

        for (i, &b) in buf.iter().enumerate() {
            match (b, start) {
                (b'\x03', None) => { return Some((i, i+1)); }
                (b'$', None) => start = Some(i),
                (b'#', Some(s)) if i+2 < buf.len() => {
                    return Some((s, i+3));
                }
                _ => {}
            }
        }
        None
    }

    pub fn send_packet(&mut self, data: &str) -> Result<()> {
        let checksum: u8 = data.bytes().fold(0u8, |sum, b| sum.wrapping_add(b));
        let packet = format!("${}#{:02X}", data, checksum);

        self.client.write_all(packet.as_bytes())?;
        self.client.flush()?;

        // println!("gdb <- {:?}", data);

        Ok(())
    }

    pub fn ack(&mut self) -> Result<()> {
        self.client.write_all(b"+")?;
        self.client.flush()?;

        Ok(())
    }
}
