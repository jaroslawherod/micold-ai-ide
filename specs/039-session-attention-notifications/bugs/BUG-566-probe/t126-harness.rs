// Appended to crates/micold-client/src/shell/desktop_notify/linux.rs for the T126 run, then removed.

// T126 (BUG-566): the real `Notifier` on a private session bus, against a stand-in service.
// Run with DBUS_SESSION_BUS_ADDRESS set to a private `dbus-daemon --session`:
//   cargo test -p micold-client t126_real_bus -- --ignored --nocapture
#[cfg(test)]
mod t126 {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;
    use zbus::blocking::{Connection, MessageIterator};

    const PATH: &str = "/org/freedesktop/Notifications";

    /// A stand-in `org.freedesktop.Notifications`: answers `Notify` with ids counted from 1.
    fn service() -> Connection {
        let conn = Connection::session().expect("bus");
        conn.request_name(NOTIFICATIONS).expect("name");
        let serving = conn.clone();
        let next = AtomicU32::new(1);
        std::thread::spawn(move || {
            for message in MessageIterator::from(&serving) {
                let Ok(message) = message else { return };
                let header = message.header();
                if header.member().map(|m| m.as_str()) == Some("Notify") {
                    let id = next.fetch_add(1, Ordering::SeqCst);
                    let reply = zbus::Message::method_return(&header)
                        .unwrap()
                        .build(&(id,))
                        .unwrap();
                    serving.send(&reply).unwrap();
                    println!("SERVICE {} Notify -> {id}", serving.unique_name().unwrap());
                }
            }
        });
        conn
    }

    fn click(from: &Connection, to: Option<&str>, id: u32) {
        from.emit_signal(to, PATH, NOTIFICATIONS, "ActionInvoked", &(id, "default"))
            .unwrap();
    }

    fn drain(rx: &mut iced::futures::channel::mpsc::UnboundedReceiver<NotifierEvent>) -> Vec<SessionId> {
        std::thread::sleep(Duration::from_millis(600));
        let mut got = Vec::new();
        while let Ok(Some(NotifierEvent::Activated { session, .. })) = rx.try_next() {
            got.push(session);
        }
        got
    }

    fn note(session: SessionId) -> DesktopNotification {
        DesktopNotification {
            title: "t".into(),
            body: "b".into(),
            project: PathBuf::from("/repo"),
            session,
        }
    }

    #[test]
    #[ignore = "needs a private session bus"]
    fn t126_real_bus() {
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        let notifier = Notifier::new(tx);
        let a = service();
        let (s1, s2, s3) = (SessionId::new(), SessionId::new(), SessionId::new());
        notifier.show(note(s1)).expect("shown");
        std::thread::sleep(Duration::from_millis(500));
        let me = notifier
            .connection
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .unique_name()
            .unwrap()
            .to_string();
        let peer = Connection::session().unwrap();
        println!("CLIENT {me} SERVICE-A {} PEER {}", a.unique_name().unwrap(), peer.unique_name().unwrap());

        click(&peer, None, 1);
        click(&peer, Some(me.as_str()), 1);
        let forged = drain(&mut rx);
        println!("forged broadcast + unicast click on id 1 -> {} events", forged.len());
        assert!(forged.is_empty());

        click(&a, None, 1);
        let real = drain(&mut rx);
        println!("service click on id 1 -> {real:?} (s1 = {s1:?})");
        assert_eq!(real, vec![s1]);

        notifier.show(note(s2)).expect("shown"); // A's id 2
        a.release_name(NOTIFICATIONS).unwrap();
        drop(a);
        let b = service();
        println!("SERVICE-B {}", b.unique_name().unwrap());
        std::thread::sleep(Duration::from_millis(300));
        click(&b, None, 2);
        let stale = drain(&mut rx);
        println!("restarted service click on old id 2 -> {} events", stale.len());
        assert!(stale.is_empty());

        notifier.show(note(s3)).expect("shown"); // B's id 1
        click(&b, None, 1);
        let fresh = drain(&mut rx);
        println!("restarted service click on its own id 1 -> {fresh:?} (s3 = {s3:?})");
        assert_eq!(fresh, vec![s3]);
        println!("T126 PASS");
    }
}
