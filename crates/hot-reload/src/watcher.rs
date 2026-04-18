use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum WatchEvent {
    SourceChanged(PathBuf),
}

pub fn start_file_watcher(
    source_path: impl AsRef<Path>,
    tx: Sender<WatchEvent>,
) -> notify::Result<RecommendedWatcher> {
    let watched = source_path.as_ref().to_path_buf();

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_))
                    || matches!(event.kind, EventKind::Create(_))
                    || matches!(event.kind, EventKind::Remove(_))
                {
                    for path in event.paths {
                        let _ = tx.send(WatchEvent::SourceChanged(path));
                    }
                }
            }
        },
        Config::default(),
    )?;

    watcher.watch(&watched, RecursiveMode::Recursive)?;
    Ok(watcher)
}
