use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum DisplayInfoError {
    #[error("failed to parse cdi3 json")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub version: u32,
    pub parameters: Vec<DisplayParameter>,
    pub parameter_groups: Vec<DisplayParameterGroup>,
    pub parts: Vec<DisplayPart>,
    pub combined_parameters: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct DisplayParameter {
    pub id: String,
    pub group_id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DisplayParameterGroup {
    pub id: String,
    pub group_id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DisplayPart {
    pub id: String,
    pub name: String,
}

impl DisplayInfo {
    /// Parse a Cubism display-info (`.cdi3.json`) document.
    pub fn parse(json_text: &str) -> Result<Self, DisplayInfoError> {
        let json: Cdi3Json = serde_json::from_str(json_text)?;
        Ok(Self {
            version: json.version,
            parameters: json
                .parameters
                .into_iter()
                .map(|p| DisplayParameter {
                    id: p.id,
                    group_id: p.group_id,
                    name: p.name,
                })
                .collect(),
            parameter_groups: json
                .parameter_groups
                .into_iter()
                .map(|g| DisplayParameterGroup {
                    id: g.id,
                    group_id: g.group_id,
                    name: g.name,
                })
                .collect(),
            parts: json
                .parts
                .into_iter()
                .map(|p| DisplayPart {
                    id: p.id,
                    name: p.name,
                })
                .collect(),
            combined_parameters: json.combined_parameters,
        })
    }
}

#[derive(Debug, Deserialize)]
struct Cdi3Json {
    #[serde(rename = "Version")]
    version: u32,
    #[serde(default, rename = "Parameters")]
    parameters: Vec<CdiParameterJson>,
    #[serde(default, rename = "ParameterGroups")]
    parameter_groups: Vec<CdiParameterGroupJson>,
    #[serde(default, rename = "Parts")]
    parts: Vec<CdiPartJson>,
    #[serde(default, rename = "CombinedParameters")]
    combined_parameters: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CdiParameterJson {
    #[serde(rename = "Id")]
    id: String,
    #[serde(default, rename = "GroupId")]
    group_id: String,
    #[serde(default, rename = "Name")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct CdiParameterGroupJson {
    #[serde(rename = "Id")]
    id: String,
    #[serde(default, rename = "GroupId")]
    group_id: String,
    #[serde(default, rename = "Name")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct CdiPartJson {
    #[serde(rename = "Id")]
    id: String,
    #[serde(default, rename = "Name")]
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn parse_mao_cdi3() -> Result<(), Box<dyn std::error::Error>> {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/mao-wgpu/assets/Mao/Mao.cdi3.json");
        let text = std::fs::read_to_string(p)?;
        let cdi = DisplayInfo::parse(&text)?;
        assert_eq!(cdi.version, 3);
        assert!(!cdi.parameters.is_empty());
        assert!(!cdi.parts.is_empty());
        Ok(())
    }
}
