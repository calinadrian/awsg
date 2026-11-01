//! A bounded socket worker; no network IO runs on the render thread.
use crate::{Client, Server, BUILD, MAX_MESSAGE, PROTOCOL};
use std::{io::ErrorKind, net::TcpStream, sync::mpsc, thread, time::Duration};
use tungstenite::{stream::MaybeTlsStream, Error, Message};

pub struct Connection {
    pub send: mpsc::SyncSender<Client>,
    pub recv: mpsc::Receiver<Result<Server, String>>,
}

pub fn connect(url: String) -> Connection {
    let (tx, rx) = mpsc::sync_channel(256);
    let (events, recv) = mpsc::sync_channel(256);
    thread::spawn(move || {
        let result = worker(&url, rx, &events);
        if let Err(error) = result {
            let _ = events.send(Err(error));
        }
    });
    Connection { send: tx, recv }
}

fn worker(
    url: &str,
    rx: mpsc::Receiver<Client>,
    events: &mpsc::SyncSender<Result<Server, String>>,
) -> Result<(), String> {
    let config = tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    let (mut socket, _) = tungstenite::client::connect_with_config(url, Some(config), 0)
        .map_err(|e| e.to_string())?;
    let tcp: &TcpStream = match socket.get_ref() {
        MaybeTlsStream::Plain(tcp) => tcp,
        MaybeTlsStream::Rustls(tls) => &tls.sock,
        _ => return Err("Unsupported TLS transport".into()),
    };
    tcp.set_read_timeout(Some(Duration::from_millis(5)))
        .map_err(|e| e.to_string())?;
    tcp.set_nodelay(true).map_err(|e| e.to_string())?;
    tcp.set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    socket
        .send(Message::Text(
            serde_json::to_string(&Client::Hello {
                protocol: PROTOCOL,
                build: BUILD.into(),
            })
            .map_err(|e| e.to_string())?
            .into(),
        ))
        .map_err(|e| e.to_string())?;
    loop {
        for _ in 0..64 {
            match rx.try_recv() {
                Ok(message) => socket
                    .send(Message::Text(
                        serde_json::to_string(&message)
                            .map_err(|e| e.to_string())?
                            .into(),
                    ))
                    .map_err(|e| e.to_string())?,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    let _ = socket.close(None);
                    return Ok(());
                }
            }
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                let message = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                events
                    .try_send(Ok(message))
                    .map_err(|_| "Co-op receive queue is full".to_string())?;
            }
            Ok(Message::Close(_)) => return Err("The co-op server closed the connection".into()),
            Ok(_) => {
                socket.flush().map_err(|e| e.to_string())?;
            }
            Err(Error::Io(e))
                if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
}
