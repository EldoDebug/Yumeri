use indexmap::IndexMap;
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum ModelSettingError {
    #[error("failed to parse model3 json")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Model3Json {
    #[serde(rename = "Version")]
    pub version: u32,
    #[serde(rename = "FileReferences")]
    pub file_references: FileReferences,
    #[serde(default, rename = "Layout")]
    pub layout: Option<IndexMap<String, f32>>,
    #[serde(default, rename = "Groups")]
    pub groups: Vec<Group>,
    #[serde(default, rename = "HitAreas")]
    pub hit_areas: Vec<HitArea>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileReferences {
    #[serde(rename = "Moc")]
    pub moc: String,
    #[serde(rename = "Textures")]
    pub textures: Vec<String>,
    #[serde(default, rename = "Physics")]
    pub physics: Option<String>,
    #[serde(default, rename = "Pose")]
    pub pose: Option<String>,
    #[serde(default, rename = "DisplayInfo")]
    pub display_info: Option<String>,
    #[serde(default, rename = "UserData")]
    pub user_data: Option<String>,
    #[serde(default, rename = "Expressions")]
    pub expressions: Vec<ExpressionRef>,
    #[serde(default, rename = "Motions")]
    pub motions: HashMap<String, Vec<MotionRef>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpressionRef {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "File")]
    pub file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MotionRef {
    #[serde(rename = "File")]
    pub file: String,
    #[serde(default, rename = "FadeInTime")]
    pub fade_in_time: Option<f32>,
    #[serde(default, rename = "FadeOutTime")]
    pub fade_out_time: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Group {
    #[serde(rename = "Target")]
    pub target: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Ids")]
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HitArea {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ModelSetting {
    pub model_dir: PathBuf,
    pub json: Model3Json,
}

#[derive(Debug, Clone)]
pub struct ResolvedMotionRef {
    pub path: PathBuf,
    pub fade_in_time: Option<f32>,
    pub fade_out_time: Option<f32>,
}

impl ModelSetting {
    /// Parse a `model3.json` document located under `model_dir`.
    pub fn parse(model_dir: PathBuf, json_text: &str) -> Result<Self, ModelSettingError> {
        let json: Model3Json = serde_json::from_str(json_text)?;
        Ok(Self { model_dir, json })
    }

    /// Resolve a `model3.json` relative path against `model_dir`.
    pub fn resolve(&self, rel: &str) -> PathBuf {
        self.model_dir.join(Path::new(rel))
    }

    /// Return the resolved `.moc3` path.
    pub fn moc_path(&self) -> PathBuf {
        self.resolve(&self.json.file_references.moc)
    }

    /// Return an iterator of resolved texture paths in declared order.
    pub fn texture_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        self.json
            .file_references
            .textures
            .iter()
            .map(|t| self.resolve(t))
    }

    /// Return the resolved physics (`.physics3.json`) path, if present.
    pub fn physics_path(&self) -> Option<PathBuf> {
        self.json
            .file_references
            .physics
            .as_deref()
            .map(|p| self.resolve(p))
    }

    /// Return the resolved pose (`.pose3.json`) path, if present.
    pub fn pose_path(&self) -> Option<PathBuf> {
        self.json
            .file_references
            .pose
            .as_deref()
            .map(|p| self.resolve(p))
    }

    /// Return the resolved user-data (`.userdata3.json`) path, if present.
    pub fn user_data_path(&self) -> Option<PathBuf> {
        self.json
            .file_references
            .user_data
            .as_deref()
            .map(|p| self.resolve(p))
    }

    /// Return the resolved display-info (`.cdi3.json`) path, if present.
    pub fn display_info_path(&self) -> Option<PathBuf> {
        self.json
            .file_references
            .display_info
            .as_deref()
            .map(|p| self.resolve(p))
    }

    /// Return the optional layout map from `model3.json`.
    pub fn layout(&self) -> Option<&IndexMap<String, f32>> {
        self.json.layout.as_ref()
    }

    /// Return an iterator of `(name, path)` for expression files.
    pub fn expressions(&self) -> impl Iterator<Item = (&str, PathBuf)> + '_ {
        self.json
            .file_references
            .expressions
            .iter()
            .map(|e| (e.name.as_str(), self.resolve(&e.file)))
    }

    /// Return an iterator of `(group, paths)` for motion files.
    pub fn motions(&self) -> impl Iterator<Item = (&str, Vec<PathBuf>)> + '_ {
        self.json.file_references.motions.iter().map(|(k, v)| {
            let paths = v.iter().map(|m| self.resolve(&m.file)).collect::<Vec<_>>();
            (k.as_str(), paths)
        })
    }

    /// Return an iterator of `(group, refs)` for motions including per-motion fade overrides.
    pub fn motions_with_fades(&self) -> impl Iterator<Item = (&str, Vec<ResolvedMotionRef>)> + '_ {
        self.json.file_references.motions.iter().map(|(k, v)| {
            let refs = v
                .iter()
                .map(|m| ResolvedMotionRef {
                    path: self.resolve(&m.file),
                    fade_in_time: m.fade_in_time,
                    fade_out_time: m.fade_out_time,
                })
                .collect::<Vec<_>>();
            (k.as_str(), refs)
        })
    }

    /// Look up a hit area by its display name and return its id, if present.
    pub fn hit_area_id(&self, name: &str) -> Option<&str> {
        self.json
            .hit_areas
            .iter()
            .find(|h| h.name == name)
            .map(|h| h.id.as_str())
    }

    /// Look up a group by name and return its ids, or an empty slice if missing.
    pub fn group_ids(&self, group_name: &str) -> &[String] {
        self.json
            .groups
            .iter()
            .find(|g| g.name == group_name)
            .map(|g| g.ids.as_slice())
            .unwrap_or(&[])
    }
}
