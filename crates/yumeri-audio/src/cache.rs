use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;

use crate::audio::Audio;
use crate::sample_format::SampleFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoadStatus {
    Loading,
    Ready,
    Failed,
}

struct CacheEntry {
    audio: Option<Audio>,
    status: LoadStatus,
}

pub struct AudioCache {
    entries: HashMap<AudioId, CacheEntry>,
    path_index: HashMap<PathBuf, AudioId>,
    next_id: u64,
    format: SampleFormat,
    sender: mpsc::Sender<(AudioId, Result<Audio, String>)>,
    receiver: mpsc::Receiver<(AudioId, Result<Audio, String>)>,
}

impl AudioCache {
    pub fn new() -> Self {
        Self::with_format(SampleFormat::F32)
    }

    pub fn with_format(format: SampleFormat) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            entries: HashMap::new(),
            path_index: HashMap::new(),
            next_id: 0,
            format,
            sender,
            receiver,
        }
    }

    fn alloc_id(&mut self) -> AudioId {
        let id = AudioId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn load(&mut self, path: impl Into<PathBuf>) -> AudioId {
        let path = path.into();
        if let Some(&id) = self.path_index.get(&path) {
            return id;
        }

        let id = self.alloc_id();
        self.entries.insert(
            id,
            CacheEntry {
                audio: None,
                status: LoadStatus::Loading,
            },
        );
        self.path_index.insert(path.clone(), id);

        let sender = self.sender.clone();
        let format = self.format;
        std::thread::spawn(move || {
            let result = Audio::load_with(&path, format).map_err(|e| e.to_string());
            let _ = sender.send((id, result));
        });

        id
    }

    pub fn insert(&mut self, audio: Audio) -> AudioId {
        let id = self.alloc_id();
        self.entries.insert(
            id,
            CacheEntry {
                audio: Some(audio),
                status: LoadStatus::Ready,
            },
        );
        id
    }

    pub fn process_pending(&mut self) {
        while let Ok((id, result)) = self.receiver.try_recv() {
            if let Some(entry) = self.entries.get_mut(&id) {
                match result {
                    Ok(audio) => {
                        entry.audio = Some(audio);
                        entry.status = LoadStatus::Ready;
                    }
                    Err(e) => {
                        log::error!("failed to load audio {:?}: {e}", id);
                        entry.status = LoadStatus::Failed;
                    }
                }
            }
        }
    }

    pub fn get(&self, id: AudioId) -> Option<&Audio> {
        self.entries.get(&id)?.audio.as_ref()
    }

    pub fn status(&self, id: AudioId) -> LoadStatus {
        self.entries
            .get(&id)
            .map_or(LoadStatus::Failed, |e| e.status)
    }

    pub fn remove(&mut self, id: AudioId) {
        self.path_index.retain(|_, v| *v != id);
        self.entries.remove(&id);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.path_index.clear();
    }
}

impl Default for AudioCache {
    fn default() -> Self {
        Self::new()
    }
}
