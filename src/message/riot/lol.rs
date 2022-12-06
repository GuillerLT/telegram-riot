use super::{
	riot_api::{
		lol::{Game, League, Participant, Player},
		GameMode, Platform, Queue, Tier,
	},
	riot_config::lol::{Message, MessageTemplate},
};

pub fn generate_messages(
	game: &Game,
	platform: Platform,
	players_participants_leagues: &[(Player, Participant, Option<League>)],
	message: &Message,
) -> Vec<String> {
	[false, true]
		.into_iter()
		.filter_map(|result| {
			let players_participants_leagues = players_participants_leagues
				.iter()
				.filter(|(_, participant, ..)| participant.win == result)
				.collect::<Vec<_>>();
			match players_participants_leagues.len() {
				0 => None,
				1 => Some(generate_message_single(
					game,
					platform,
					result,
					players_participants_leagues.first().unwrap(),
					&message.single,
				)),
				2..=5 => Some(generate_message_multiple(
					game,
					platform,
					result,
					&players_participants_leagues,
					&message.multiple,
				)),
				n => {
					tracing::warn!(
						game = game.metadata.match_id,
						"Trying to generate a message from a RIOT LOL game with {n} tracked participants"
					);
					None
				}
			}
		})
		.collect()
}

fn generate_message_single(
	game: &Game,
	platform: Platform,
	result: bool,
	(player, participant, league): &(Player, Participant, Option<League>),
	message_template: &MessageTemplate,
) -> String {
	substitute_common(
		game,
		platform,
		result,
		match (result, league) {
			(true, None) => &message_template.win_single,
			(true, Some(_)) => &message_template.win_single_ranked,
			(false, None) => &message_template.loss_single,
			(false, Some(_)) => &message_template.loss_single_ranked,
		},
	)
	.replace("{sumoner_name}", &player.name)
	.replace("{champion}", &participant.champion_name)
	.replace("{kills}", &format!("{}", participant.kills))
	.replace("{deaths}", &format!("{}", participant.deaths))
	.replace("{assists}", &format!("{}", participant.assists))
	.replace(
		"{damage}",
		&format!("{}", participant.total_damage_dealt_to_champions),
	)
	.replace(
		"{damage_percentage}",
		&format!("{:.1}", get_damage_percentage(game, result, participant)),
	)
	.replace(
		"{tier}",
		league
			.as_ref()
			.and_then(|league| league.tier)
			.unwrap_or(Tier::UNRANKED)
			.as_ref(),
	)
	.replace(
		"{rank}",
		&league
			.as_ref()
			.and_then(|league| league.rank)
			.map_or_else(String::default, |rank| rank.to_string()),
	)
	.replace(
		"{lp}",
		&format!(
			"{}",
			league.as_ref().map_or(-1, |league| league.league_points)
		),
	)
}

fn generate_message_multiple(
	game: &Game,
	platform: Platform,
	result: bool,
	players_participants_leagues: &[&(Player, Participant, Option<League>)],
	message_template: &MessageTemplate,
) -> String {
	substitute_common(
		game,
		platform,
		result,
		if result {
			&message_template.win_multiple
		} else {
			&message_template.loss_multiple
		},
	)
	.replace("{sumoner_names}", &{
		// TODO: interperse interperse // iter_intersperse #79524 // https://github.com/rust-lang/rust/issues/79524
		let mut player_names = players_participants_leagues
			.iter()
			.map(|(player, ..)| &player.name);
		let first = player_names.next().unwrap();
		player_names.fold(String::from(first), |mut player_names, player_name| {
			player_names.push_str(" &amp; ");
			player_names.push_str(player_name);
			player_names
		})
	})
	.replace(
		"{singles}",
		&players_participants_leagues
			.iter()
			.map(|player_participant_league| {
				generate_message_single(
					game,
					platform,
					result,
					player_participant_league,
					message_template,
				)
			})
			.collect::<String>(),
	)
}

fn substitute_common(game: &Game, platform: Platform, _result: bool, message: &str) -> String {
	message
		.replace("{mode}", &get_queue_or_mode_string(game))
		.replace(
			"{game_duration_min}",
			&format!("{}", game.info.game_duration / 60),
		)
		.replace("{region}", platform.as_region_str())
}

fn get_queue_or_mode_string(game: &Game) -> String {
	match (&game.info.game_mode, game.info.queue_id) {
		(GameMode::CLASSIC, Queue::SUMMONERS_RIFT_5V5_RANKED_SOLO) => String::from("RANKED"),
		(GameMode::CLASSIC, Queue::SUMMONERS_RIFT_5V5_RANKED_FLEX) => String::from("RANKED FLEX"),
		(GameMode::CLASSIC, Queue::CUSTOM) => String::from("CUSTOM"),
		(GameMode::CLASSIC, ..) => String::from("NORMAL"),
		(mode, ..) => mode.to_string(),
	}
}

fn get_damage_percentage(game: &Game, result: bool, participant: &Participant) -> f64 {
	100.0 * f64::from(participant.total_damage_dealt_to_champions)
		/ f64::from(
			game.info
				.participants
				.iter()
				.filter(|participant| participant.win == result)
				.map(|participant| participant.total_damage_dealt_to_champions)
				.sum::<i32>()
				.max(1),
		)
}
