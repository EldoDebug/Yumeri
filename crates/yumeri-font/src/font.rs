use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{FontError, Result};

static NEXT_FONT_ID: AtomicU64 = AtomicU64::new(0);

pub struct Font {
    pub(crate) inner: cosmic_text::FontSystem,
    id: u64,
}

impl Font {
    pub fn new() -> Self {
        Self {
            inner: cosmic_text::FontSystem::new(),
            id: NEXT_FONT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn with_locale(locale: &str) -> Self {
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_system_fonts();
        Self {
            inner: cosmic_text::FontSystem::new_with_locale_and_db(locale.to_string(), db),
            id: NEXT_FONT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    #[doc(hidden)]
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn load_font(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        self.inner
            .db_mut()
            .load_font_file(path)
            .map_err(|source| FontError::Io {
                path: path.to_path_buf(),
                source,
            })
    }

    pub fn load_font_data(&mut self, data: Vec<u8>) -> Result<()> {
        let count_before = self.inner.db().len();
        self.inner.db_mut().load_font_data(data);
        if self.inner.db().len() == count_before {
            return Err(FontError::InvalidFontData);
        }
        Ok(())
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font").finish_non_exhaustive()
    }
}
