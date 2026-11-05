//! Offline-first reconciliation. The acknowledgement is journalled before either save changes.
use super::*;

impl Equipment {
    fn cache_local(&mut self, inventory: &Inventory) {
        self.import_profile = true;
        for (key, hero) in &mut self.save.heroes {
            if key.starts_with("solo:") {
                let class = hero.inventory.class;
                hero.inventory = inventory.clone();
                hero.inventory.class = class;
            }
        }
    }

    pub(super) fn finish_delivery(&mut self) -> bool {
        let Some(delivery) = self.save.delivery.clone() else {
            return true;
        };
        let Some(remote) = delivery.reply.inventory.clone() else {
            return false;
        };
        let class = remote.class;
        let Some(local) = self.view(SOLO, class) else {
            return false;
        };
        let Some(mut ledger) = self.ledger.take() else {
            // Load the local ledger, but do not execute additional jobs during recovery.
            let state = solo_path()
                .and_then(|p| crate::local_state::read_to_string(&p).ok())
                .and_then(|s| serde_json::from_str::<LedgerState>(&s).ok());
            if let Some(state) = state {
                self.ledger = Some(Ledger::new(state, Catalog::load()));
            }
            return false;
        };
        let delivery_id = &delivery.job.request.id;
        let result = if let Some(inventory) = ledger.delivered_snapshot(&local.token, delivery_id) {
            Ok(inventory)
        } else {
            let base = delivery.job.local_base.as_ref().unwrap_or(&local.inventory);
            asr_coop::offline::merge(&remote, base, &local.inventory, &self.catalog).and_then(
                |inventory| {
                    ledger.install_snapshot(
                        &local.token,
                        inventory,
                        delivery_id,
                        &mut SoloHost::new(),
                    )
                },
            )
        };
        self.ledger = Some(ledger);
        match result {
            Ok(inventory) => {
                self.cache_local(&inventory);
                self.save.sync_bases.insert(delivery.server, remote);
                self.listings = delivery.reply.listings;
                self.save
                    .pending
                    .retain(|job| job.request.id != *delivery_id);
                self.save.delivery = None;
                self.selected = None;
                self.status = "Inventory and offline progress synchronized".into();
                self.revision += 1;
                self.persist()
            }
            Err(error) => {
                self.status =
                    format!("{error}. Both saves are preserved; synchronization is paused.");
                self.revision += 1;
                false
            }
        }
    }

    fn namespace_local(&mut self, class: u8) -> bool {
        if self.save.namespaced {
            return true;
        }
        let Some(hero) = self.view(SOLO, class) else {
            return false;
        };
        // Preserve both original stores before the first migration; never overwrite backups.
        for source in [path(), solo_path()].into_iter().flatten() {
            let backup = source.with_extension("before-offline-sync.bak");
            if crate::local_state::read_to_string(&backup).is_ok() {
                continue;
            }
            if let Ok(text) = crate::local_state::read_to_string(&source) {
                if let Err(error) = crate::local_state::write_atomic(&backup, &text) {
                    self.status = format!("Could not back up inventory before sync: {error}");
                    return false;
                }
            }
        }
        let Some(mut ledger) = self.ledger.take() else {
            return false;
        };
        let mut inventory = hero.inventory;
        let mut host = SoloHost::new();
        let prefix = ((u64::from(host.random(u32::MAX)) << 32) | u64::from(host.random(u32::MAX))) & 0x0FFF_FFFF_FFFF_FFFF;
        for (index, item) in inventory.items.iter_mut().enumerate() {
            item.id = prefix + index as u64;
        }
        let result = ledger.install_snapshot(&hero.token, inventory, "namespace-v1", &mut host);
        self.ledger = Some(ledger);
        match result {
            Ok(inventory) => {
                self.cache_local(&inventory);
                self.save.namespaced = true;
                self.selected = None;
                self.revision += 1;
                self.persist()
            }
            Err(error) => {
                self.status = error;
                false
            }
        }
    }

    pub(super) fn sync_profile(
        &mut self,
        class: u8,
        talents: &mut super::super::talents::TalentBook,
        progress: &mut super::super::progress::Progress,
    ) {
        let Some(hero) = self.view(SOLO, class) else {
            return;
        };
        let mut inventory = hero.inventory.clone();
        if self.import_profile {
            talents.import_offline(&inventory.offline_progress);
            progress.import_offline(&inventory);
            self.import_profile = false;
        }
        talents.export_offline(&mut inventory.offline_progress);
        progress.export_offline(&mut inventory);
        if asr_coop::offline::same_inventory(&inventory, &hero.inventory) {
            return;
        }
        let Some(mut ledger) = self.ledger.take() else {
            return;
        };
        let result = ledger.install_snapshot(
            &hero.token,
            inventory,
            &format!("profile:{}", id()),
            &mut SoloHost::new(),
        );
        self.ledger = Some(ledger);
        match result {
            Ok(inventory) => {
                self.cache_local(&inventory);
                self.import_profile = false;
                self.persist();
                self.revision += 1;
            }
            Err(error) => {
                self.status = error;
                self.read_only = true;
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn sync_network(&mut self, _url: &str, _name: &str, _class: u8, _dt: f32) {}

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn sync_network(&mut self, url: &str, name: &str, class: u8, dt: f32) {
        if url.is_empty() {
            return;
        }
        if self.save.device.is_empty() {
            self.save.device = SoloHost::new().secret();
            if !self.persist() {
                return;
            }
        }
        self.timer += dt;
        self.refresh_t += dt;
        if self.server != url {
            self.connection = None;
            self.connected = false;
            self.flight = false;
            self.server = url.into();
            self.link_failed = false;
            self.listings.clear();
            self.timer = 3.;
        }
        if self.connection.is_none() && self.timer >= 3. {
            self.timer = 0.;
            self.connection = Some(std::sync::Mutex::new(asr_coop::transport::connect(
                url.into(),
            )));
        }
        let events: Vec<_> = self
            .connection
            .as_ref()
            .and_then(|c| c.lock().ok())
            .map_or_else(Vec::new, |c| c.recv.try_iter().take(64).collect());
        for event in events {
            match event {
                Ok(Server::Welcome) => {
                    self.connected = true;
                    self.timer = 0.;
                    self.revision += 1;
                }
                Ok(Server::Economy { reply }) => {
                    let Some(job) = self
                        .save
                        .pending
                        .first()
                        .filter(|j| self.flight && j.request.id == reply.id)
                        .cloned()
                    else {
                        continue;
                    };
                    self.flight = false;
                    if reply.retry
                        || (!reply.ok && matches!(job.request.action, Action::SyncOffline { .. }))
                    {
                        self.status = format!("Offline progress retained: {}", reply.message);
                        self.connection = None;
                        self.connected = false;
                        self.timer = -12.;
                        self.revision += 1;
                        continue;
                    }
                    let apply_local = reply.ok
                        && (matches!(job.request.action, Action::SyncOffline { .. })
                            || (job.local_base.is_some() && reply.inventory.is_some()));
                    if apply_local {
                        self.save.delivery = Some(Delivery {
                            server: url.into(),
                            job,
                            reply,
                        });
                        if !self.persist() || !self.finish_delivery() {
                            return;
                        }
                    } else {
                        self.save.pending.remove(0);
                        self.receive(url, job, reply);
                    }
                }
                Err(reason) | Ok(Server::Error { reason }) => {
                    self.status =
                        format!("Offline: progress saved locally. Reconnecting… {reason}");
                    self.connection = None;
                    self.connected = false;
                    self.flight = false;
                    self.timer = 0.;
                    self.revision += 1;
                }
                _ => {}
            }
        }
        if self.flight && self.timer > 15. {
            self.connection = None;
            self.connected = false;
            self.flight = false;
            self.timer = 0.;
            self.status = "Offline: progress saved locally; retrying synchronization".into();
            self.revision += 1;
        }
        if !self.connected || self.flight || self.save.delivery.is_some() {
            return;
        }
        self.ensure_hero(url, class, name);
        self.link_classes(url, class);
        let prefix = format!("{}:", url.trim_end_matches('/'));
        let relevant = self.save.pending.iter().any(|j| j.key.starts_with(&prefix));
        if !relevant
            && !self.save.pending.iter().any(|j| j.key.starts_with("solo:"))
            && self.namespace_local(class)
        {
            if let Some(local) = self.view(SOLO, class) {
                let base = self.save.sync_bases.get(url).cloned().unwrap_or_default();
                let action = if !asr_coop::offline::same_inventory(&base, &local.inventory) {
                    Some(Action::SyncOffline {
                        device: self.save.device.clone(),
                        base: Box::new(base),
                        inventory: Box::new(local.inventory.clone()),
                    })
                } else if self.refresh_t > 10. {
                    Some(Action::Online {
                        device: self.save.device.clone(),
                        action: Box::new(Action::Refresh),
                    })
                } else {
                    None
                };
                if let Some(action) = action {
                    self.refresh_t = 0.;
                    self.queue(Self::key(url, class), action);
                    if let Some(job) = self.save.pending.last_mut() {
                        job.local_base = Some(local.inventory);
                    }
                    if !self.persist() {
                        return;
                    }
                }
            }
        }
        if let Some(index) = self
            .save
            .pending
            .iter()
            .position(|j| j.key.starts_with(&prefix))
        {
            if index != 0 {
                let job = self.save.pending.remove(index);
                self.save.pending.insert(0, job);
            }
            let Some(request) = self.signed(&self.save.pending[0]) else {
                self.status = "Server credential missing; offline inventory is preserved".into();
                return;
            };
            self.save.pending[0].request = request;
            if !self.persist() {
                return;
            }
            let request = self.save.pending[0].request.clone();
            self.flight = self
                .connection
                .as_ref()
                .and_then(|c| c.lock().ok())
                .is_some_and(|c| c.send.try_send(Client::Economy { request }).is_ok());
            self.timer = 0.;
        }
    }
}
