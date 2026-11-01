//! Run with `cargo run --release -p asr-coop -- --bind 0.0.0.0:47771`.
#[cfg(not(target_arch = "wasm32"))]
mod directory {
    use asr_coop::{Client, Room, Server, BUILD, MAX_MESSAGE, MAX_PLAYERS, PROTOCOL};
    use std::{
        collections::HashMap,
        io::ErrorKind,
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc, Mutex,
        },
        time::{Duration, Instant},
    };
    use tungstenite::{Error, Message};

    struct Player {
        out: mpsc::SyncSender<Server>,
        failed: Arc<AtomicBool>,
        hello: bool,
        room: Option<String>,
        peer: u32,
    }
    struct Match {
        room: Room,
        running: bool,
        next: u32,
        host: u64,
        players: HashMap<u32, u64>,
    }
    #[derive(Default)]
    struct Directory {
        players: HashMap<u64, Player>,
        rooms: HashMap<String, Match>,
        serial: u64,
    }

    impl Directory {
        fn send(&self, id: u64, message: Server) {
            if let Some(p) = self.players.get(&id) {
                if p.out.try_send(message).is_err() {
                    p.failed.store(true, Ordering::Relaxed);
                }
            }
        }
        fn error(&self, id: u64, reason: &str) {
            self.send(
                id,
                Server::Error {
                    reason: reason.into(),
                },
            );
        }
        fn room_changed(&self, code: &str) {
            if let Some(m) = self.rooms.get(code) {
                for (&peer, &id) in &m.players {
                    self.send(
                        id,
                        Server::Room {
                            room: m.room.clone(),
                            peer,
                        },
                    );
                }
            }
        }
        fn leave(&mut self, id: u64) {
            let Some(p) = self.players.get_mut(&id) else {
                return;
            };
            let Some(code) = p.room.take() else { return };
            let peer = p.peer;
            if peer == 0 {
                if let Some(m) = self.rooms.remove(&code) {
                    for other in m.players.values() {
                        if let Some(p) = self.players.get_mut(other) {
                            p.room = None;
                        }
                        self.send(
                            *other,
                            Server::Closed {
                                reason: "The host closed the party".into(),
                            },
                        );
                    }
                }
            } else {
                if let Some(m) = self.rooms.get_mut(&code) {
                    m.players.remove(&peer);
                    m.room.members.retain(|p| p.peer != peer);
                    let host = m.host;
                    self.send(host, Server::Left { peer });
                }
                self.send(
                    id,
                    Server::Closed {
                        reason: "You left the party".into(),
                    },
                );
                self.room_changed(&code);
            }
        }
        fn handle(&mut self, id: u64, message: Client) {
            let Some(player) = self.players.get(&id) else {
                return;
            };
            if let Client::Hello { protocol, build } = message {
                if player.hello || protocol != PROTOCOL || build != BUILD {
                    self.error(id, "This server requires the same ASR-game build");
                    return;
                }
                if let Some(p) = self.players.get_mut(&id) {
                    p.hello = true;
                }
                self.send(id, Server::Welcome);
                return;
            }
            if !player.hello {
                self.error(id, "Handshake required");
                return;
            }
            match message {
                Client::List => self.send(
                    id,
                    Server::Rooms {
                        rooms: self
                            .rooms
                            .values()
                            .filter(|m| !m.room.private && !m.running)
                            .map(|m| m.room.clone())
                            .collect(),
                    },
                ),
                Client::Create {
                    name,
                    private,
                    map,
                    mut member,
                } => {
                    if player.room.is_some() || self.rooms.len() >= 64 {
                        self.error(id, "Leave your current room before hosting");
                        return;
                    }
                    if name.len() > 64 || !valid_member(&member) || map != 0 {
                        self.error(id, "Invalid room or hero");
                        return;
                    }
                    // Random codes are unguessable enough for friend invitations; they are not account credentials.
                    let code = loop {
                        let code = tungstenite::handshake::client::generate_key()
                            .chars()
                            .filter(|c| c.is_ascii_alphanumeric())
                            .take(6)
                            .collect::<String>()
                            .to_uppercase();
                        if !self.rooms.contains_key(&code) {
                            break code;
                        }
                    };
                    member.peer = 0;
                    member.ready = false;
                    let room = Room {
                        code: code.clone(),
                        name,
                        map,
                        private,
                        members: vec![member],
                    };
                    self.rooms.insert(
                        code.clone(),
                        Match {
                            room,
                            running: false,
                            next: 1,
                            host: id,
                            players: HashMap::from([(0, id)]),
                        },
                    );
                    if let Some(p) = self.players.get_mut(&id) {
                        p.room = Some(code.clone());
                        p.peer = 0;
                    }
                    self.room_changed(&code);
                }
                Client::Join { code, mut member } => {
                    if player.room.is_some() || !valid_member(&member) {
                        self.error(id, "Leave your room before joining");
                        return;
                    }
                    let code = code.to_uppercase();
                    let Some(m) = self.rooms.get_mut(&code) else {
                        self.error(id, "Room not found");
                        return;
                    };
                    if m.running || m.players.len() >= MAX_PLAYERS {
                        self.error(id, "The room is full or already playing");
                        return;
                    }
                    let peer = m.next;
                    m.next += 1;
                    member.peer = peer;
                    member.ready = false;
                    m.players.insert(peer, id);
                    m.room.members.push(member);
                    if let Some(p) = self.players.get_mut(&id) {
                        p.room = Some(code.clone());
                        p.peer = peer;
                    }
                    self.room_changed(&code);
                }
                Client::Leave => self.leave(id),
                Client::Ready { ready } => {
                    let Some(code) = player.room.clone() else {
                        return;
                    };
                    let peer = player.peer;
                    if let Some(m) = self.rooms.get_mut(&code) {
                        if m.running {
                            return;
                        }
                        if let Some(p) = m.room.members.iter_mut().find(|p| p.peer == peer) {
                            p.ready = ready;
                        }
                    }
                    self.room_changed(&code);
                }
                Client::Start => {
                    let Some(code) = player.room.clone() else {
                        return;
                    };
                    let Some(m) = self.rooms.get_mut(&code) else {
                        return;
                    };
                    if m.host != id || m.running || !m.room.members.iter().all(|p| p.ready) {
                        self.error(id, "Everyone must be ready; only the host can start");
                        return;
                    }
                    m.running = true;
                    let members: Vec<_> = m.players.values().copied().collect();
                    for other in members {
                        self.send(other, Server::Start);
                    }
                }
                Client::Data { payload } => {
                    let Some(code) = player.room.as_ref() else {
                        return;
                    };
                    let Some(m) = self.rooms.get(code) else {
                        return;
                    };
                    if !m.running {
                        return;
                    }
                    let peer = player.peer;
                    if peer == 0 {
                        for (&p, &other) in &m.players {
                            if p != 0 {
                                self.send(
                                    other,
                                    Server::Data {
                                        peer,
                                        payload: payload.clone(),
                                    },
                                );
                            }
                        }
                    } else {
                        self.send(m.host, Server::Data { peer, payload });
                    }
                }
                Client::Hello { .. } => {}
            }
        }
    }
    fn valid_member(p: &asr_coop::Member) -> bool {
        !p.name.trim().is_empty()
            && p.name.len() <= 32
            && !p.name.chars().any(char::is_control)
            && p.hero.class < 9
            && p.hero.sex <= 1
            && p.hero.display > 0
            && (1..=100).contains(&p.hero.race)
    }
    fn client(
        stream: TcpStream,
        state: Arc<Mutex<Directory>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        stream.set_nodelay(true)?;
        let config = tungstenite::protocol::WebSocketConfig::default()
            .max_message_size(Some(MAX_MESSAGE))
            .max_frame_size(Some(MAX_MESSAGE));
        let mut socket = tungstenite::accept_with_config(stream, Some(config))?;
        socket
            .get_ref()
            .set_read_timeout(Some(Duration::from_millis(5)))?;
        let (tx, rx) = mpsc::sync_channel(256);
        let failed = Arc::new(AtomicBool::new(false));
        let id = {
            let mut d = state.lock().map_err(|_| "Directory lock failed")?;
            if d.players.len() >= 256 {
                return Err("Server full".into());
            }
            d.serial += 1;
            let id = d.serial;
            d.players.insert(
                id,
                Player {
                    out: tx,
                    failed: failed.clone(),
                    hello: false,
                    room: None,
                    peer: 0,
                },
            );
            id
        };
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut window = Instant::now();
            let mut count = 0;
            let mut bytes = 0usize;
            let mut last = Instant::now();
            loop {
                if failed.load(Ordering::Relaxed) {
                    return Err("Connection could not keep up with the match".into());
                }
                for message in rx.try_iter() {
                    socket.send(Message::Text(serde_json::to_string(&message)?.into()))?;
                }
                match socket.read() {
                    Ok(Message::Text(text)) => {
                        if window.elapsed() >= Duration::from_secs(1) {
                            window = Instant::now();
                            count = 0;
                            bytes = 0;
                        }
                        count += 1;
                        bytes += text.len();
                        last = Instant::now();
                        if count > 120 || bytes > 8 * MAX_MESSAGE {
                            return Err("Rate limit exceeded".into());
                        }
                        let message = serde_json::from_str::<Client>(&text)?;
                        state
                            .lock()
                            .map_err(|_| "Directory lock failed")?
                            .handle(id, message);
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {
                        socket.flush()?;
                    }
                    Err(Error::Io(e))
                        if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) => {}
                    Err(e) => return Err(e.into()),
                }
                if last.elapsed() > Duration::from_secs(30) {
                    return Err("Client timed out".into());
                }
            }
            Ok(())
        })();
        if let Ok(mut d) = state.lock() {
            d.leave(id);
            d.players.remove(&id);
        }
        result
    }
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let args: Vec<_> = std::env::args().collect();
        let bind = args
            .windows(2)
            .find(|p| p[0] == "--bind")
            .map_or("127.0.0.1:47771", |p| p[1].as_str());
        let listener = TcpListener::bind(bind)?;
        println!("ASR-game lobby and relay: {bind}; build {BUILD}; four players per room");
        let state = Arc::new(Mutex::new(Directory::default()));
        for stream in listener.incoming() {
            let stream = stream?;
            let state = state.clone();
            if state
                .lock()
                .map_err(|_| "Directory lock failed")?
                .players
                .len()
                >= 256
            {
                continue;
            }
            std::thread::spawn(move || {
                if let Err(e) = client(stream, state) {
                    eprintln!("co-op: {e}");
                }
            });
        }
        Ok(())
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    directory::run()
}
#[cfg(target_arch = "wasm32")]
fn main() {}
