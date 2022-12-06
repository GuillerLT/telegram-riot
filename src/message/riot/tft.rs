use super::{
	riot_api::{
		tft::{Game, League, Participant, Player},
		Platform, Queue, Tier,
	},
	riot_config::tft::{Message, MessageTemplate},
};

pub fn generate_messages(
	game: &Game,
	platform: Platform,
	players_participants_leagues: &[(Player, Participant, Option<League>)],
	message: &Message,
) -> Vec<String> {
	let is_double = game.info.queue_id
		== i32::from(Queue::CONVERGENCE_RANKED_TEAMFIGHT_TACTICS_DOUBLE_UP_WORKSHOP_.0);

	(1..=(if is_double { 4 } else { 8 }))
		.into_iter()
		.filter_map(|result| {
			let players_participants_leagues = players_participants_leagues
				.iter()
				.filter(|(_, participant, _)| if is_double { (participant.placement + 1) / 2} else {participant.placement} == result)
				.collect::<Vec<_>>();
			match players_participants_leagues.len() {
				0 => None,
				1 => Some(generate_message_single(
					game,
					platform,
					result,
					players_participants_leagues.first().unwrap(),
					&message.single,
					if is_double { 2 } else { 4 }
				)),
				2 => Some(generate_message_duo(
					game,
					platform,
					result,
					players_participants_leagues.first().unwrap(),
					players_participants_leagues.last().unwrap(),
					&message.duo,
					if is_double { 2 } else { 4 }
				)),
				n => {
					tracing::warn!(
						game = game.metadata.match_id,
						"Trying to generate a message from a RIOT TFT game with {n} tracked participants"
					);
					None
				}
			}
		}).collect()
}

fn generate_message_single(
	game: &Game,
	platform: Platform,
	result: i32,
	(player, .., league): &(Player, Participant, Option<League>),
	message_template: &MessageTemplate,
	threshold: i32,
) -> String {
	substitute_common(
		game,
		platform,
		result,
		if result <= threshold {
			match league {
				None => &message_template.top_single,
				Some(_) => &message_template.top_single_ranked,
			}
		} else {
			match league {
				None => &message_template.bottom_single,
				Some(_) => &message_template.bottom_single_ranked,
			}
		},
	)
	.replace("{sumoner_name}", &player.name)
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
			league
				.as_ref()
				.and_then(|league| league.league_points)
				.unwrap_or(-1)
		),
	)
}

fn generate_message_duo(
	game: &Game,
	platform: Platform,
	result: i32,
	player_participant_league_a: &(Player, Participant, Option<League>),
	player_participant_league_b: &(Player, Participant, Option<League>),
	message_template: &MessageTemplate,
	threshold: i32,
) -> String {
	let (player_a, ..) = player_participant_league_a;
	let (player_b, ..) = player_participant_league_b;
	substitute_common(
		game,
		platform,
		result,
		if result <= threshold {
			&message_template.top_duo
		} else {
			&message_template.bottom_duo
		},
	)
	.replace(
		"{sumoner_names}",
		&format!("{} &amp; {}", player_a.name, player_b.name),
	)
	.replace("{singles}", &{
		generate_message_single(
			game,
			platform,
			result,
			player_participant_league_a,
			message_template,
			threshold,
		) + &generate_message_single(
			game,
			platform,
			result,
			player_participant_league_b,
			message_template,
			threshold,
		)
	})
}

fn substitute_common(game: &Game, platform: Platform, result: i32, message: &str) -> String {
	message
		.replace("{mode}", &get_queue_or_mode_string(game))
		.replace("{top}", &format!("{result}"))
		.replace(
			"{game_duration_min}",
			&format!("{:.0}", game.info.game_length / 60.0),
		)
		.replace("{region}", platform.as_region_str())
}

fn get_queue_or_mode_string(game: &Game) -> String {
	let queue_id = u16::try_from(game.info.queue_id).unwrap_or_else(|err| {
		tracing::error!(
			error = err.to_string(),
			"Error converting to queue from identifier"
		);
		0
	});

	match Queue(queue_id) {
		Queue::CONVERGENCE_RANKED_TEAMFIGHT_TACTICS => String::from("RANKED"),
		Queue::CONVERGENCE_RANKED_TEAMFIGHT_TACTICS_HYPER_ROLL_ => String::from("HYPER ROLL"),
		Queue::CONVERGENCE_RANKED_TEAMFIGHT_TACTICS_DOUBLE_UP_WORKSHOP_ => {
			String::from("DOUBLE UP")
		}
		_ => String::from("NORMAL"),
	}
}
