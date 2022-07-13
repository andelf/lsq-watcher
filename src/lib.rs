use std::path::PathBuf;

pub mod fsevent;

pub enum EventKind {
    Created,
    Deleted,
    Updated,
    Renamed,
}

pub struct Event {
    paths: Vec<PathBuf>,
    kind: EventKind,
}
pub struct WatcherOptions {}
