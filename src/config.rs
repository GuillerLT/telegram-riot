use serde::{Deserialize, Serialize};

pub mod riot;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
	pub telegram_api_key: String,
	pub riot_lol_api_key: String,
	pub riot_tft_api_key: String,
	pub riot_lol_message: riot::lol::Message,
	pub riot_tft_message: riot::tft::Message,
	pub trackers: Vec<Tracker>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Tracker {
	#[serde(rename = "telegram-chat")]
	pub telegram_chat: i64,
	#[serde(default, rename = "riot-lol-players")]
	pub riot_lol_platforms_names: std::collections::BTreeMap<String, Vec<String>>,
	#[serde(default, rename = "riot-tft-players")]
	pub riot_tft_platforms_names: std::collections::BTreeMap<String, Vec<String>>,
}
