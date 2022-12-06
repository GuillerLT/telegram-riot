use std::collections::{BTreeMap, BTreeSet};

mod api;
mod config;
mod db;
mod message;

#[tokio::main(flavor = "current_thread")]
async fn main() {
	tracing_subscriber::fmt::fmt()
		.with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
		.with_timer(tracing_subscriber::fmt::time::UtcTime::new(
			time::format_description::parse(
				"[year repr:last_two]-[month]-[day]T[hour]:[minute]:[second]",
			)
			.unwrap(),
		))
		.init();

	// Load configuration from config.json
	let config: config::Config =
		serde_json::de::from_str(&tokio::fs::read_to_string("config.json").await.unwrap()).unwrap();

	// Connect to SQLite DB
	let db_pool = db::SqlitePool::connect("riot.sqlite").await.unwrap();
	tokio::try_join!(
		db::riot::lol::create_tables(&db_pool),
		db::riot::tft::create_tables(&db_pool),
	)
	.unwrap();

	// Get RIOT names-platforms and Telegram chats
	let mut telegram_chats = BTreeSet::default();
	let mut lol_names_platforms_telegram_chats = BTreeMap::<_, BTreeSet<_>>::default();
	let mut tft_names_platforms_telegram_chats = BTreeMap::<_, BTreeSet<_>>::default();
	for config::Tracker {
		telegram_chat,
		riot_lol_platforms_names: lol_platforms_names,
		riot_tft_platforms_names: tft_platforms_names,
	} in config.trackers
	{
		let telegram_chat = api::telegram::ChatId(telegram_chat);
		telegram_chats.insert(telegram_chat);
		for (platform, names) in lol_platforms_names {
			let platform: api::riot::Platform = platform.to_uppercase().parse().unwrap();
			for name in names {
				lol_names_platforms_telegram_chats
					.entry((name, platform))
					.or_default()
					.insert(telegram_chat);
			}
		}
		for (platform, names) in tft_platforms_names {
			let platform: api::riot::Platform = platform.to_uppercase().parse().unwrap();
			for name in names {
				tft_names_platforms_telegram_chats
					.entry((name, platform))
					.or_default()
					.insert(telegram_chat);
			}
		}
	}
	let telegram_chats = Vec::from_iter(telegram_chats);
	let lol_names_platforms_telegram_chats = lol_names_platforms_telegram_chats;
	let tft_names_platforms_telegram_chats = tft_names_platforms_telegram_chats;

	// RIOT API instances
	let lol_api = std::sync::Arc::new(api::riot::Api::new(config.riot_lol_api_key));
	let tft_api = std::sync::Arc::new(api::riot::Api::new(config.riot_tft_api_key));

	// LOL player getter task
	let lol_get_players = {
		let api = lol_api.clone();
		async move {
			let mut players_platforms_telegram_chats = Vec::default();
			let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
			for ((name, platform), telegram_chats) in lol_names_platforms_telegram_chats {
				interval.tick().await;
				let player = api::riot::lol::get_player(&api, platform, &name)
					.await
					.unwrap()
					.unwrap();
				let telegram_chats = Vec::from_iter(telegram_chats);
				players_platforms_telegram_chats.push(((player, platform), telegram_chats));
			}
			players_platforms_telegram_chats
		}
	};

	// TFT player getter task
	let tft_get_players = {
		let api = tft_api.clone();
		async move {
			let mut players_platforms_telegram_chats = Vec::default();
			let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
			for ((name, platform), telegram_chats) in tft_names_platforms_telegram_chats {
				interval.tick().await;
				let player = api::riot::tft::get_player(&api, platform, &name)
					.await
					.unwrap()
					.unwrap();
				let telegram_chats = Vec::from_iter(telegram_chats);
				players_platforms_telegram_chats.push(((player, platform), telegram_chats));
			}
			players_platforms_telegram_chats
		}
	};

	// Get players
	let (lol_players_platforms_telegram_chats, tft_players_platforms_telegram_chats) =
		tokio::join!(lol_get_players, tft_get_players);

	// Store players in DB
	tokio::try_join!(
		db::riot::lol::insert_players(&db_pool, &lol_players_platforms_telegram_chats),
		db::riot::tft::insert_players(&db_pool, &tft_players_platforms_telegram_chats),
	)
	.unwrap();

	// LOL game identifiers getter task
	let (lol_game_ids_sender, mut lol_game_ids_receiver) = tokio::sync::mpsc::channel(128);
	let lol_get_game_ids = async {
		let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(4));
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
		for ((player, platform), ..) in lol_players_platforms_telegram_chats.iter().cycle() {
			interval.tick().await;
			let game_ids = api::riot::lol::get_last_game_ids(&lol_api, *platform, player)
				.await
				.unwrap_or_default();

			for game_id in game_ids {
				lol_game_ids_sender
					.send((game_id, *platform))
					.await
					.unwrap_or_else(|err| {
						tracing::error!(
							error = err.to_string(),
							"Error sending LOL game identifier to channel"
						)
					});
			}
			tokio::task::yield_now().await;
		}
	};

	// TFT game identifiers getter task
	let (tft_game_ids_sender, mut tft_game_ids_receiver) = tokio::sync::mpsc::channel(128);
	let tft_get_game_ids = async {
		let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(4));
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
		for ((player, platform), ..) in tft_players_platforms_telegram_chats.iter().cycle() {
			interval.tick().await;
			let game_ids = api::riot::tft::get_last_game_ids(&tft_api, *platform, player)
				.await
				.unwrap_or_default();

			for game_id in game_ids {
				tft_game_ids_sender
					.send((game_id, *platform))
					.await
					.unwrap_or_else(|err| {
						tracing::error!(
							error = err.to_string(),
							"Error sending TFT game identifier to channel"
						)
					});
			}
			tokio::task::yield_now().await;
		}
	};

	let (messages_sender, mut messages_receiver) = tokio::sync::mpsc::unbounded_channel();

	// LOL game getter task
	let lol_get_games = async {
		let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
		while let Some((game_id, platform)) = lol_game_ids_receiver.recv().await {
			if db::riot::lol::contains_game(&db_pool, &game_id, platform)
				.await
				.unwrap_or(true)
			{
				continue;
			}

			interval.tick().await;
			let Ok(Some(game)) =
					api::riot::lol::get_game(&lol_api, platform, &game_id).await
			else {
				continue
			};

			let players_participants_telegram_chats =
				game.info.participants.iter().filter_map(|participant| {
					lol_players_platforms_telegram_chats
						.iter()
						.find(|((player, ..), ..)| player.puuid == participant.puuid)
						.map(|((player, ..), telegram_chats)| (player, participant, telegram_chats))
				});

			let mut players_participants_leagues_telegram_chats = Vec::default();
			for (player, participant, telegram_chats) in players_participants_telegram_chats {
				interval.tick().await;
				let league = api::riot::lol::get_leagues(&lol_api, platform, player)
					.await
					.unwrap_or_default()
					.into_iter()
					.find(|league| {
						api::riot::are_same_queue(&league.queue_type, game.info.queue_id)
					});

				players_participants_leagues_telegram_chats.push((
					player,
					participant,
					league,
					telegram_chats,
				));
			}
			let players_participants_leagues_telegram_chats =
				players_participants_leagues_telegram_chats;

			if db::riot::lol::insert_game(
				&db_pool,
				&game,
				platform,
				&players_participants_leagues_telegram_chats,
			)
			.await
			.is_err()
			{
				continue;
			}

			for telegram_chat in telegram_chats.iter().copied() {
				let players_participants_leagues = players_participants_leagues_telegram_chats
					.iter()
					.filter(|(.., telegram_chats)| telegram_chats.contains(&telegram_chat))
					.map(|(player, participant, league, ..)| {
						((*player).clone(), (*participant).clone(), league.clone())
					})
					.collect::<Vec<_>>();
				let messages = message::riot::lol::generate_messages(
					&game,
					platform,
					&players_participants_leagues,
					&config.riot_lol_message,
				);
				for message in messages {
					messages_sender
						.send((telegram_chat, message))
						.unwrap_or_else(|err| {
							tracing::error!(
								error = err.to_string(),
								"Error sending Telegram message (LOL) to channel"
							)
						});
				}
			}
		}

		tracing::error!("Riot LOL game identifier receiver has closed unexpectedly");
	};

	// TFT game getter task
	let tft_get_games = async {
		let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
		while let Some((game_id, platform)) = tft_game_ids_receiver.recv().await {
			if db::riot::tft::contains_game(&db_pool, &game_id, platform)
				.await
				.unwrap_or(true)
			{
				continue;
			}

			interval.tick().await;
			let Ok(Some(game)) =
					api::riot::tft::get_game(&tft_api, platform, &game_id).await
			else {
				continue
			};

			let players_participants_telegram_chats =
				game.info.participants.iter().filter_map(|participant| {
					tft_players_platforms_telegram_chats
						.iter()
						.find(|((player, ..), ..)| player.puuid == participant.puuid)
						.map(|((player, ..), telegram_chats)| (player, participant, telegram_chats))
				});

			let mut players_participants_leagues_telegram_chats = Vec::default();
			for (player, participant, telegram_chats) in players_participants_telegram_chats {
				interval.tick().await;
				let league = api::riot::tft::get_leagues(&tft_api, platform, player)
					.await
					.unwrap_or_default()
					.into_iter()
					.find(|league| {
						api::riot::are_same_queue_id(&league.queue_type, game.info.queue_id)
					});

				players_participants_leagues_telegram_chats.push((
					player,
					participant,
					league,
					telegram_chats,
				));
			}
			let players_participants_leagues_telegram_chats =
				players_participants_leagues_telegram_chats;

			if db::riot::tft::insert_game(
				&db_pool,
				&game,
				platform,
				&players_participants_leagues_telegram_chats,
			)
			.await
			.is_err()
			{
				continue;
			}

			for telegram_chat in telegram_chats.iter().copied() {
				let players_participants_leagues = players_participants_leagues_telegram_chats
					.iter()
					.filter(|(.., telegram_chats)| telegram_chats.contains(&telegram_chat))
					.map(|(player, participant, league, ..)| {
						((*player).clone(), (*participant).clone(), league.clone())
					})
					.collect::<Vec<_>>();

				let messages = message::riot::tft::generate_messages(
					&game,
					platform,
					&players_participants_leagues,
					&config.riot_tft_message,
				);
				for message in messages {
					messages_sender
						.send((telegram_chat, message))
						.unwrap_or_else(|err| {
							tracing::error!(
								error = err.to_string(),
								"Error sending Telegram message (TFT) to channel"
							)
						});
				}
			}
		}

		tracing::error!("Riot TFT game identifier receiver has closed unexpectedly");
	};

	// Telegram notifier task
	let telegram_notify = async {
		// Telegram API instance
		let telegram_api = api::telegram::Throttle::new_spawn(
			api::telegram::Api::new(config.telegram_api_key),
			api::telegram::Limits::default(),
		);

		while let Some((telegram_chat, message)) = messages_receiver.recv().await {
			if api::telegram::send_message(&telegram_api, telegram_chat, &message)
				.await
				.is_err()
			{
				messages_sender
					.send((telegram_chat, message))
					.unwrap_or_else(|err| {
						tracing::error!(
							error = err.to_string(),
							"Error resending Telegram message to channel"
						)
					});
			}
		}

		tracing::error!("Telegram message receiver has closed unexpectedly");
	};

	// Run tasks
	tokio::select! {
		_ = lol_get_game_ids => {},
		_ = tft_get_game_ids => {},
		_ = lol_get_games => {},
		_ = tft_get_games => {},
		_ = telegram_notify => {},
		signal = tokio::signal::ctrl_c() => {
			signal.unwrap_or_else(|err| {
				tracing::error!(error = err.to_string(), "Error handling CTRL-C signal")
			})
		},
	};

	tracing::debug!("Exiting");
}
