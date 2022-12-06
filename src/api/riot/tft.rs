use super::{Api, Platform, Result, LAST_COUNT};

pub use riven::models::{
	tft_league_v1::LeagueEntry as League,
	tft_match_v1::{Match as Game, Participant},
	tft_summoner_v1::Summoner as Player,
};

pub async fn get_player(api: &Api, platform: Platform, name: &str) -> Result<Option<Player>> {
	api.tft_summoner_v1()
		.get_by_summoner_name(platform, name)
		.await
		.map_err(|err| {
			// TODO: inspect_err // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			tracing::error!(
				platform = platform.as_region_str(),
				player = name,
				error = err.source_reqwest_error().to_string(),
				response = err.status_code().map(|err| err.to_string()),
				"Error getting Riot TFT player"
			);
			err
		})
		.map(|player| {
			// TODO: inspect // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			player
				.or_else(|| {
					tracing::error!(
						platform = platform.as_region_str(),
						player = name,
						"Error getting Riot TFT player"
					);
					None
				})
				.map(|player| {
					// TODO: inspect // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
					tracing::debug!(
						platform = platform.as_region_str(),
						player = player.name,
						puuid = player.puuid,
						"Success getting Riot TFT player"
					);
					player
				})
		})
}

pub async fn get_last_game_ids(
	api: &Api,
	platform: Platform,
	player: &Player,
) -> Result<Vec<String>> {
	api.tft_match_v1()
		.get_match_ids_by_puuid(
			platform.to_regional(),
			&player.puuid,
			Some(LAST_COUNT),
			None,
			None,
			None,
		)
		.await
		.map_err(|err| {
			// TODO: inspect_err // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			tracing::error!(
				platform = platform.as_region_str(),
				player = player.name,
				error = err.source_reqwest_error().to_string(),
				response = err.status_code().map(|err| err.to_string()),
				"Error getting Riot TFT game identifiers"
			);
			err
		})
		.map(|game_ids| {
			// TODO: inspect // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			tracing::trace!(
				platform = platform.as_region_str(),
				player = player.name,
				n = game_ids.len(),
				"Success downloading Riot TFT game identifiers"
			);
			game_ids
		})
}

pub async fn get_game(api: &Api, platform: Platform, game_id: &str) -> Result<Option<Game>> {
	api.tft_match_v1()
		.get_match(platform.to_regional(), game_id)
		.await
		.map_err(|err| {
			// TODO: inspect_err // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			tracing::error!(
				platform = platform.as_region_str(),
				game = game_id,
				error = err.source_reqwest_error().to_string(),
				response = err.status_code().map(|err| err.to_string()),
				"Error getting Riot TFT game"
			);
			err
		})
		.map(|game| {
			// TODO: inspect // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			game.map(|game| {
				// TODO: inspect // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345

				let datetime_to_string = |datetime: chrono::naive::NaiveDateTime| {
					datetime.format("%y-%m-%dT%H:%M:%S").to_string()
				};

				let end =
					chrono::naive::NaiveDateTime::from_timestamp_millis(game.info.game_datetime)
						.map(datetime_to_string);

				let start = chrono::naive::NaiveDateTime::from_timestamp_millis(
					game.info.game_datetime - (game.info.game_length * 1000.0) as i64,
				)
				.map(datetime_to_string);

				tracing::debug!(
					platform = platform.as_region_str(),
					game = game.metadata.match_id,
					start,
					end,
					"Success getting Riot TFT game"
				);
				game
			})
		})
}

pub async fn get_leagues(api: &Api, platform: Platform, player: &Player) -> Result<Vec<League>> {
	api.tft_league_v1()
		.get_league_entries_for_summoner(platform, &player.id)
		.await
		.map_err(|err| {
			// TODO: inspect_err // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			tracing::error!(
				platform = platform.as_region_str(),
				player = player.name,
				error = err.source_reqwest_error().to_string(),
				response = err.status_code().map(|err| err.to_string()),
				"Error getting Riot TFT leagues"
			);
			err
		})
}
