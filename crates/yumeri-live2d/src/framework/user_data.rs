use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum UserDataError {
    #[error("failed to parse userdata3 json")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct UserData {
    nodes: Vec<UserDataNode>,
    art_mesh_nodes: Vec<UserDataNode>,
}

#[derive(Debug, Clone)]
pub struct UserDataNode {
    pub target: String,
    pub id: String,
    pub value: String,
}

impl UserData {
    /// Parse a user-data (`.userdata3.json`) document.
    pub fn parse(json_text: &str) -> Result<Self, UserDataError> {
        let json: UserData3Json = serde_json::from_str(json_text)?;
        let nodes = json
            .user_data
            .unwrap_or_default()
            .into_iter()
            .map(|n| UserDataNode {
                target: n.target,
                id: n.id,
                value: n.value,
            })
            .collect::<Vec<_>>();

        let art_mesh_nodes = nodes
            .iter()
            .filter(|n| n.target == "ArtMesh")
            .cloned()
            .collect::<Vec<_>>();

        Ok(Self {
            nodes,
            art_mesh_nodes,
        })
    }

    /// Return all user-data nodes.
    pub fn nodes(&self) -> &[UserDataNode] {
        &self.nodes
    }

    /// Return only user-data nodes targeting `ArtMesh`.
    pub fn art_mesh_nodes(&self) -> &[UserDataNode] {
        &self.art_mesh_nodes
    }
}

#[derive(Debug, Deserialize)]
struct UserData3Json {
    #[serde(default, rename = "UserData")]
    user_data: Option<Vec<UserDataNodeJson>>,
}

#[derive(Debug, Deserialize)]
struct UserDataNodeJson {
    #[serde(rename = "Target")]
    target: String,
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Value")]
    value: String,
}
