//! The realm service's ledger host. The directory serializes transactions. Two alternating,
//! synced snapshots preserve the previous generation if a write is interrupted, on Windows and
//! Unix. The rules themselves are [`crate::ledger`]'s.
use crate::economy::*;
use crate::ledger::{Host, Ledger, State};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Store {
    path: PathBuf,
    ledger: Ledger,
}
/// The service's clock, dice and disk.
struct Disk<'a> {
    path: &'a Path,
}
fn secret() -> String {
    format!(
        "{}{}",
        tungstenite::handshake::client::generate_key(),
        tungstenite::handshake::client::generate_key()
    )
}
impl Host for Disk<'_> {
    fn backup_before_sync(&mut self, state: &[u8]) -> Result<(), String> {
        let path = self.path.with_extension("before-offline-sync.json");
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(mut file) => file.write_all(state).and_then(|_| file.sync_all()).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let bytes = std::fs::read(self.path.with_extension("before-offline-sync.json")).map_err(|e| e.to_string())?;
                serde_json::from_slice::<State>(&bytes).map(|_| ()).map_err(|e| format!("Pre-sync backup is unreadable: {e}"))
            }
            Err(e) => Err(e.to_string()),
        }
    }
    fn now(&mut self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
    fn random(&mut self, limit: u32) -> u32 {
        if limit == 0 {
            return 0;
        }
        let key = secret();
        let value = key
            .bytes()
            .fold(0u64, |v, c| v.wrapping_mul(257).wrapping_add(u64::from(c)));
        (value % u64::from(limit)) as u32
    }
    fn secret(&mut self) -> String {
        secret()
    }
    fn save(&mut self, generation: u64, state: &[u8]) -> Result<(), String> {
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let path = self.path.with_extension(format!("{}.json", generation % 2));
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(state)
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("Could not save transaction: {e}"))?;
        #[cfg(unix)]
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
impl Store {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let mut states = Vec::new();
        let mut existed = false;
        for slot in [0, 1] {
            let p = path.with_extension(format!("{slot}.json"));
            match std::fs::read(&p) {
                Ok(bytes) => {
                    existed = true;
                    if let Ok(state) = serde_json::from_slice::<State>(&bytes) {
                        states.push(state);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Cannot read economy store: {e}")),
            }
        }
        if existed && states.is_empty() {
            return Err("Both economy snapshots are unreadable; restore a backup".into());
        }
        let state = states
            .into_iter()
            .max_by_key(State::generation)
            .unwrap_or_default();
        Ok(Self {
            path,
            ledger: Ledger::new(state, Catalog::load()),
        })
    }
    pub fn handle(&mut self, request: Request) -> Reply {
        self.ledger.handle(request, &mut Disk { path: &self.path })
    }
}
