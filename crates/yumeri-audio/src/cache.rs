use std::collections::HashMap;
use std::path::PathBuf;

use yumeri_threading::{Task, TaskStatus, ThreadPool};

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
    pending_loads: Vec<(AudioId, Task<Result<Audio, String>>)>,
    next_id: u64,
    format: SampleFormat,
}

impl AudioCache {
    pub fn new() -> Self {
        Self::with_format(SampleFormat::F32)
    }

    pub fn with_format(format: SampleFormat) -> Self {
        Self {
            entries: HashMap::new(),
            path_index: HashMap::new(),
            pending_loads: Vec::new(),
            next_id: 0,
            format,
        }
    }

    fn alloc_id(&mut self) -> AudioId {
        let id = AudioId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn load(&mut self, pool: &ThreadPool, path: impl Into<PathBuf>) -> AudioId {
        let path = path.into();
        if let Some(&id) = self.path_index.get(&path) {
            return id;
        }

        let id = self.alloc_id();
        self.path_index.insert(path.clone(), id);

        let format = self.format;
        let task = pool.spawn_task(move || {
            Audio::load_with(&path, format).map_err(|e| e.to_string())
        });

        self.entries.insert(
            id,
            CacheEntry {
                audio: None,
                status: LoadStatus::Loading,
            },
        );
        self.pending_loads.push((id, task));

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
        let mut i = 0;
        while i < self.pending_loads.len() {
            self.pending_loads[i].1.poll();
            match self.pending_loads[i].1.status() {
                TaskStatus::Ready => {
                    let (id, mut task) = self.pending_loads.swap_remove(i);
                    let result = task.take().unwrap();
                    if let Some(entry) = self.entries.get_mut(&id) {
                        match result {
                            Ok(audio) => {
                                entry.audio = Some(audio);
                                entry.status = LoadStatus::Ready;
                            }
                            Err(e) => {
                                log::error!("failed to load audio {id:?}: {e}");
                                entry.status = LoadStatus::Failed;
                            }
                        }
                    }
                }
                TaskStatus::Failed => {
                    let (id, _) = self.pending_loads.swap_remove(i);
                    log::error!("audio load task {id:?} failed");
                    if let Some(entry) = self.entries.get_mut(&id) {
                        entry.status = LoadStatus::Failed;
                    }
                }
                TaskStatus::Pending => {
                    i += 1;
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
        self.pending_loads.retain(|(pid, _)| *pid != id);
        self.entries.remove(&id);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.path_index.clear();
        self.pending_loads.clear();
    }
}

impl Default for AudioCache {
    fn default() -> Self {
        Self::new()
    }
}
