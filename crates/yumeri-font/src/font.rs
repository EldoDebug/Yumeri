use std::fmt;
use std::path::Path;

use crate::error::{FontError, Result};

pub struct Font {
    pub(crate) inner: cosmic_text::FontSystem,
}

impl Font {
    pub fn new() -> Self {
        Self {
            inner: cosmic_text::FontSystem::new(),
        }
    }

    pub fn with_locale(locale: &str) -> Self {
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_system_fonts();
        Self {
            inner: cosmic_text::FontSystem::new_with_locale_and_db(locale.to_string(), db),
        }
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
