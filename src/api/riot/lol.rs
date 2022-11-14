use super::{Api, Platform, Result, LAST_COUNT};

pub use riven::models::{
	league_v4::LeagueEntry as League,
	match_v5::{Match as Game, Participant},
	summoner_v4::Summoner as Player,
};

pub async fn get_player(api: &Api, platform: Platform, name: &str) -> Result<Option<Player>> {
	api.summoner_v4()
		.get_by_summoner_name(platform, name)
		.await
		.map_err(|err| {
			// TODO: inspect_err // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			tracing::error!(
				platform = platform.as_region_str(),
				player = name,
				error = err.source_reqwest_error().to_string(),
				response = err.status_code().map(|err| err.to_string()),
				"Error getting Riot LOL player"
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
						"Error getting Riot LOL player"
					);
					None
				})
				.map(|player| {
					// TODO: inspect // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
					tracing::info!(
						platform = platform.as_region_str(),
						player = player.name,
						puuid = player.puuid,
						"Success getting Riot LOL player"
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
	api.match_v5()
		.get_match_ids_by_puuid(
			platform.to_regional(),
			&player.puuid,
			Some(LAST_COUNT),
			None,
			None,
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
				"Error getting Riot LOL game IDs"
			);
			err
		})
}

pub async fn get_game(api: &Api, platform: Platform, game_id: &str) -> Result<Option<Game>> {
	api.match_v5()
		.get_match(platform.to_regional(), game_id)
		.await
		.map_err(|err| {
			// TODO: inspect_err // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			tracing::error!(
				platform = platform.as_region_str(),
				game = game_id,
				error = err.source_reqwest_error().to_string(),
				response = err.status_code().map(|err| err.to_string()),
				"Error getting Riot LOL game"
			);
			err
		})
		.map(|game| {
			// TODO: inspect // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			game.map(|game| {
				// TODO: inspect // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
				tracing::info!(
					platform = platform.as_region_str(),
					game = game.metadata.match_id,
					"Success getting Riot LOL game"
				);
				game
			})
		})
}

pub async fn get_leagues(api: &Api, platform: Platform, player: &Player) -> Result<Vec<League>> {
	api.league_v4()
		.get_league_entries_for_summoner(platform, &player.id)
		.await
		.map_err(|err| {
			// TODO: inspect_err // result_option_inspect #91345 // https://github.com/rust-lang/rust/issues/91345
			tracing::error!(
				platform = platform.as_region_str(),
				player = player.name,
				error = err.source_reqwest_error().to_string(),
				response = err.status_code().map(|err| err.to_string()),
				"Error getting Riot LOL leagues"
			);
			err
		})
}
