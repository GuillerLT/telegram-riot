use super::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Message {
	pub single: MessageTemplate,
	pub duo: MessageTemplate,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct MessageTemplate {
	pub top_single: String,
	pub top_single_ranked: String,
	pub top_duo: String,
	pub bottom_single: String,
	pub bottom_single_ranked: String,
	pub bottom_duo: String,
}
