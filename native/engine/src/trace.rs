//! A small trace of what the hooks heard and did, in `config/chestcavity-trace.log` beside `options.txt`.
//! The game's own log says nothing of a mod's hooks, and what a hook cannot do fails quietly, so this is
//! how to see why a click did nothing. It stays under 200 KB: past that it starts over.
use horizon_client::mod_api::Host;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_BYTES: u64 = 200_000;

pub fn line(host: &dyn Host, text: impl AsRef<str>) {
    let Ok(dir) = host.config_dir() else { return };
    let path = dir.join("chestcavity-trace.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = std::fs::remove_file(&path);
    }
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{secs} {}", text.as_ref());
    }
}
