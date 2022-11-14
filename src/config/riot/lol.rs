use super::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Message {
	pub single: MessageTemplate,
	pub multiple: MessageTemplate,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct MessageTemplate {
	pub win_single: String,
	pub win_single_ranked: String,
	pub win_multiple: String,
	pub loss_single: String,
	pub loss_single_ranked: String,
	pub loss_multiple: String,
}
