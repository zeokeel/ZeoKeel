//! Connector aggregate contract (`srv.connectors`).
//!
//! @capability contract.connector
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// MCP transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorTransport { Stdio, Http, Sse }

/// Connector definition (no secret/token).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Connector {
    pub id: String,
    pub name: String,
    pub transport: ConnectorTransport,
    pub auth_mode: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub enabled: bool,
}

/// Connector grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConnectorGrant {
    pub grantee: String,
    pub can_enable: bool,
}

/// OAuth public-client metadata (no client_secret).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConnectorOAuth {
    pub client_id: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub redirect_uri: String,
}
