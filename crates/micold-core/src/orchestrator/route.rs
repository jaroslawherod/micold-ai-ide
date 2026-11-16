//! The orchestrator's route table (#1726, data-model): which client conversation is which link
//! conversation, and which requests on each have no answer yet. Pure: no socket, no clock.
//!
//! A client names a conversation by `(client, daemon, conn)`; the table gives it a link conn on
//! that daemon's link. The two directions are a bijection. Link conns are allocated per daemon and
//! never reused, even after a route closes, so a late frame for a closed route cannot reach a new
//! one. Conn `0` of every link belongs to the orchestrator itself (its list queries).

use std::collections::{BTreeSet, HashMap};

use crate::protocol::routing::{ConnectionId, WireDaemonId};

/// A client connection of the orchestrator.
pub type ClientId = u64;

/// A route's client side.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientKey {
    /// The client connection.
    pub client: ClientId,
    /// The daemon the client addresses.
    pub daemon: WireDaemonId,
    /// The client's conn for that daemon.
    pub conn: ConnectionId,
}

/// A route dropped from the table, with the requests that never got an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedRoute {
    /// The client side.
    pub key: ClientKey,
    /// The link conn it used.
    pub link_conn: u64,
    /// Requests forwarded and not answered, in ascending order.
    pub pending: Vec<u64>,
}

#[derive(Debug, Default)]
struct Link {
    next_conn: u64,
    by_link: HashMap<u64, ClientKey>,
}

/// The table.
#[derive(Debug, Default)]
pub struct RouteTable {
    by_client: HashMap<ClientKey, u64>,
    links: HashMap<WireDaemonId, Link>,
    pending: HashMap<(WireDaemonId, u64), BTreeSet<u64>>,
}

impl RouteTable {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// The link conn of `key`, opening a route when there is none. The flag is `true` when it was
    /// just opened.
    pub fn open(&mut self, key: &ClientKey) -> (u64, bool) {
        if let Some(conn) = self.by_client.get(key) {
            return (*conn, false);
        }
        let link = self.links.entry(key.daemon.clone()).or_default();
        link.next_conn += 1;
        let conn = link.next_conn;
        link.by_link.insert(conn, key.clone());
        self.by_client.insert(key.clone(), conn);
        (conn, true)
    }

    /// The link conn of `key`, if its route is open.
    pub fn link_conn(&self, key: &ClientKey) -> Option<u64> {
        self.by_client.get(key).copied()
    }

    /// The client side of `link_conn` on `daemon`'s link.
    pub fn client_of(&self, daemon: &WireDaemonId, link_conn: u64) -> Option<&ClientKey> {
        self.links.get(daemon)?.by_link.get(&link_conn)
    }

    /// Note that request `req` was forwarded on the route.
    pub fn forwarded(&mut self, daemon: &WireDaemonId, link_conn: u64, req: u64) {
        if self.client_of(daemon, link_conn).is_some() {
            self.pending
                .entry((daemon.clone(), link_conn))
                .or_default()
                .insert(req);
        }
    }

    /// Note that `req` was answered. A `req` that is not pending is ignored.
    pub fn answered(&mut self, daemon: &WireDaemonId, link_conn: u64, req: u64) {
        let key = (daemon.clone(), link_conn);
        if let Some(set) = self.pending.get_mut(&key) {
            set.remove(&req);
            if set.is_empty() {
                self.pending.remove(&key);
            }
        }
    }

    /// Close one route, returning what it held. `None` when it was not open.
    pub fn close(&mut self, key: &ClientKey) -> Option<DroppedRoute> {
        let link_conn = self.by_client.remove(key)?;
        if let Some(link) = self.links.get_mut(&key.daemon) {
            link.by_link.remove(&link_conn);
        }
        let pending = self
            .pending
            .remove(&(key.daemon.clone(), link_conn))
            .map(|set| set.into_iter().collect())
            .unwrap_or_default();
        Some(DroppedRoute {
            key: key.clone(),
            link_conn,
            pending,
        })
    }

    /// Close every route of `client` (it left), in ascending order of daemon, then link conn.
    pub fn close_client(&mut self, client: ClientId) -> Vec<DroppedRoute> {
        let keys: Vec<_> = self
            .by_client
            .keys()
            .filter(|k| k.client == client)
            .cloned()
            .collect();
        let mut dropped: Vec<_> = keys.iter().filter_map(|k| self.close(k)).collect();
        dropped.sort_by(|a, b| (&a.key.daemon, a.link_conn).cmp(&(&b.key.daemon, b.link_conn)));
        dropped
    }

    /// Close every route on `daemon`'s link (it dropped), in ascending link conn order. The link's
    /// conn counter is kept, so a daemon that links again never meets an old conn id.
    pub fn drop_link(&mut self, daemon: &WireDaemonId) -> Vec<DroppedRoute> {
        let keys: Vec<_> = self
            .by_client
            .keys()
            .filter(|k| &k.daemon == daemon)
            .cloned()
            .collect();
        let mut dropped: Vec<_> = keys.iter().filter_map(|k| self.close(k)).collect();
        dropped.sort_by_key(|d| d.link_conn);
        dropped
    }

    /// How many routes are open.
    pub fn len(&self) -> usize {
        self.by_client.len()
    }

    /// Whether no route is open.
    pub fn is_empty(&self) -> bool {
        self.by_client.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> WireDaemonId {
        WireDaemonId::new(s).unwrap()
    }

    fn key(client: ClientId, daemon: &str, conn: u64) -> ClientKey {
        ClientKey {
            client,
            daemon: id(daemon),
            conn: ConnectionId(conn),
        }
    }

    #[test]
    fn the_two_directions_are_a_bijection() {
        let mut table = RouteTable::new();
        let a = key(1, "a", 7);
        let b = key(2, "a", 7);
        let (la, fresh) = table.open(&a);
        assert!(fresh);
        let (lb, _) = table.open(&b);
        assert_ne!(la, lb);
        assert_eq!(table.open(&a), (la, false));
        assert_eq!(table.client_of(&id("a"), la), Some(&a));
        assert_eq!(table.client_of(&id("a"), lb), Some(&b));
        assert_eq!(table.link_conn(&a), Some(la));
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn one_client_conn_to_two_daemons_is_two_routes() {
        let mut table = RouteTable::new();
        let (x, _) = table.open(&key(1, "a", 7));
        let (y, _) = table.open(&key(1, "b", 7));
        assert_eq!(table.client_of(&id("a"), x), Some(&key(1, "a", 7)));
        assert_eq!(table.client_of(&id("b"), y), Some(&key(1, "b", 7)));
    }

    #[test]
    fn link_conns_start_above_the_orchestrators_own_and_are_never_reused() {
        let mut table = RouteTable::new();
        let k = key(1, "a", 1);
        let (first, _) = table.open(&k);
        assert!(first >= 1, "conn 0 is the orchestrator's");
        table.close(&k).unwrap();
        let (second, fresh) = table.open(&k);
        assert!(fresh);
        assert!(second > first);
        assert_eq!(table.client_of(&id("a"), first), None);
    }

    #[test]
    fn an_answer_clears_its_request_and_is_idempotent() {
        let mut table = RouteTable::new();
        let k = key(1, "a", 1);
        let (conn, _) = table.open(&k);
        table.forwarded(&id("a"), conn, 5);
        table.forwarded(&id("a"), conn, 6);
        table.answered(&id("a"), conn, 5);
        table.answered(&id("a"), conn, 5);
        table.answered(&id("a"), conn, 99);
        let dropped = table.close(&k).unwrap();
        assert_eq!(dropped.pending, vec![6]);
    }

    #[test]
    fn a_request_on_no_route_is_not_remembered() {
        let mut table = RouteTable::new();
        table.forwarded(&id("a"), 3, 1);
        let k = key(1, "a", 1);
        table.open(&k);
        assert!(table.close(&k).unwrap().pending.is_empty());
    }

    #[test]
    fn drop_link_returns_every_route_and_pending_request_of_that_daemon_only() {
        let mut table = RouteTable::new();
        let (c1, _) = table.open(&key(1, "a", 1));
        let (c2, _) = table.open(&key(2, "a", 1));
        let (other, _) = table.open(&key(1, "b", 1));
        table.forwarded(&id("a"), c1, 9);
        table.forwarded(&id("a"), c1, 4);
        table.forwarded(&id("b"), other, 1);
        let dropped = table.drop_link(&id("a"));
        assert_eq!(dropped.len(), 2);
        assert_eq!(dropped[0].link_conn, c1);
        assert_eq!(dropped[0].pending, vec![4, 9]);
        assert_eq!(dropped[1].link_conn, c2);
        assert!(dropped[1].pending.is_empty());
        assert_eq!(table.len(), 1);
        assert!(table.drop_link(&id("a")).is_empty());
        // The daemon links again: its conns do not repeat.
        let (again, _) = table.open(&key(1, "a", 1));
        assert!(again > c2);
    }

    #[test]
    fn close_client_closes_all_its_routes_and_none_of_anothers() {
        let mut table = RouteTable::new();
        table.open(&key(1, "a", 1));
        table.open(&key(1, "b", 1));
        table.open(&key(2, "a", 1));
        let dropped = table.close_client(1);
        assert_eq!(dropped.len(), 2);
        assert_eq!(table.len(), 1);
        assert!(table.link_conn(&key(2, "a", 1)).is_some());
    }
}
