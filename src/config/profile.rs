use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Profile {
    #[serde(default)]
    pub bind: Vec<BindConfig>,
}

#[derive(Deserialize, Serialize)]
pub struct BindConfig {
    #[serde(default)]
    pub layer: Option<String>,
    pub key: KeySpec,
    #[serde(default)]
    pub action: Option<Action>,
    #[serde(rename = "action-up", default)]
    pub action_up: Option<Action>,
    #[serde(default)]
    pub binding: Option<BindingSpec>,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum KeySpec {
    String(String),
    Number(u32),
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Exec(ExecAction),
    Http(HttpAction),
    Cell(CellAction),
    Timer(TimerAction),
    Layer(LayerAction),
    LayerGroup(LayerGroupAction),
}

#[derive(Deserialize, Serialize)]
pub struct ExecAction {
    pub path: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, EnvValue>,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum EnvValue {
    String(String),
    Bool(bool),
}

#[derive(Deserialize, Serialize)]
pub struct HttpAction {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub query: Option<toml::Value>,
    #[serde(default)]
    pub body: Option<toml::Value>,
}

#[derive(Deserialize, Serialize)]
pub struct CellAction {
    pub cell: String,
    pub command: CellCommand,
    #[serde(default)]
    pub value: Option<i64>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CellCommand {
    Increment,
    Decrement,
    Reset,
    Set,
}

#[derive(Deserialize, Serialize)]
pub struct TimerAction {
    #[serde(rename = "cell")]
    pub timer: String,
    pub command: TimerCommand,
    #[serde(default)]
    pub value: Option<u64>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerCommand {
    Start,
    Stop,
    Restart,
}

#[derive(Deserialize, Serialize)]
pub struct LayerAction {
    pub layer: String,
    pub command: LayerCommand,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerCommand {
    Active,
    Inactive,
}

#[derive(Deserialize, Serialize)]
pub struct LayerGroupAction {
    pub group: String,
    pub command: LayerGroupCommand,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerGroupCommand {
    Clear,
    Next,
    Previous,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum BindingSpec {
    Cell(BindingCell),
    Timer(BindingTimer),
}

#[derive(Deserialize, Serialize)]
pub struct BindingCell {
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    #[serde(default)]
    pub filter: Option<BindingFilter>,
}

#[derive(Deserialize, Serialize)]
pub struct BindingTimer {
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum BindingFilter {
    Number(i64),
    Command(CellCommand),
}
