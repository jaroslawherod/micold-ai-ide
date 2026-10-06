// Repro for #566: which ActionInvoked signals does a zbus MessageIterator on this match rule pass on?
use std::time::Duration;
fn main() {
    let rule = std::env::args().nth(1).unwrap();
    if rule == "own" {
        let c = zbus::blocking::connection::Builder::session().unwrap().name("org.freedesktop.Notifications").unwrap().build().unwrap();
        println!("OWNER {}", c.unique_name().unwrap()); use std::io::Write; std::io::stdout().flush().unwrap();
        std::thread::sleep(Duration::from_millis(1500));
        c.emit_signal(None::<&str>, "/org/freedesktop/Notifications", "org.freedesktop.Notifications", "ActionInvoked", &(1u32, "default")).unwrap();
        std::thread::sleep(Duration::from_secs(3)); return;
    }
    let conn = zbus::blocking::connection::Builder::session().unwrap().build().unwrap();
    println!("UNIQUE {}", conn.unique_name().unwrap());
    let it = zbus::blocking::MessageIterator::for_match_rule(rule.as_str(), &conn, None).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || for m in it { let m = m.unwrap(); let h = m.header();
        tx.send(format!("GOT member={:?} sender={:?} dest={:?}", h.member().map(|x| x.to_string()), h.sender().map(|x| x.to_string()), h.destination().map(|x| x.to_string()))).unwrap(); });
    use std::io::Write; std::io::stdout().flush().unwrap();
    while let Ok(l) = rx.recv_timeout(Duration::from_secs(4)) { println!("{l}"); std::io::stdout().flush().unwrap(); }
}
