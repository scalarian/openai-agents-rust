use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    pub name: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Button {
    pub label: String,
    pub action: Option<String>,
}

/// Computer-use environment metadata.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Computer {
    pub environment: Option<Environment>,
    pub display_name: Option<String>,
    #[serde(default)]
    pub buttons: Vec<Button>,
}

pub type AsyncComputer = Computer;
