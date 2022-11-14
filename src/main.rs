use std::collections::{BTreeMap, BTreeSet};

mod api;
mod config;
mod db;
mod message;

#[tokio::main(flavor = "current_thread")]
async fn main() {
	tracing_subscriber::fmt::init();

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

	// RIOT API instances
	let lol_api = std::sync::Arc::new(api::riot::Api::new(config.riot_lol_api_key));
	let tft_api = std::sync::Arc::new(api::riot::Api::new(config.riot_tft_api_key));

	// LOL player getter task
	let lol_get_players = {
		let lol_api = lol_api.clone();
		async move {
			let mut lol_players_platforms_telegram_chats = Vec::default();
			let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
			for ((name, platform), telegram_chats) in lol_names_platforms_telegram_chats {
				interval.tick().await;
				let player = api::riot::lol::get_player(&lol_api, platform, &name)
					.await
					.unwrap()
					.unwrap();
				let telegram_chats = Vec::from_iter(telegram_chats);
				lol_players_platforms_telegram_chats.push(((player, platform), telegram_chats));
			}
			lol_players_platforms_telegram_chats
		}
	};

	// TFT player getter task
	let tft_get_players = {
		let tft_api = tft_api.clone();
		async move {
			let mut tft_players_platforms_telegram_chats = Vec::default();
			let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
			for ((name, platform), telegram_chats) in tft_names_platforms_telegram_chats {
				interval.tick().await;
				let player = api::riot::tft::get_player(&tft_api, platform, &name)
					.await
					.unwrap()
					.unwrap();
				let telegram_chats = Vec::from_iter(telegram_chats);
				tft_players_platforms_telegram_chats.push(((player, platform), telegram_chats));
			}
			tft_players_platforms_telegram_chats
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
	let (lol_game_ids_sender, mut lol_game_ids_receiver) = tokio::sync::mpsc::channel(64);
	let lol_get_game_ids = async {
		let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
		for ((player, platform), ..) in lol_players_platforms_telegram_chats.iter().cycle() {
			interval.tick().await;
			let Ok(lol_game_ids) =
					api::riot::lol::get_last_game_ids(&lol_api, *platform, player).await
			else {
				continue
			};

			for lol_game_id in lol_game_ids {
				lol_game_ids_sender
					.send((lol_game_id, *platform))
					.await
					.unwrap_or_default();
			}
		}
	};

	// TFT game identifiers getter task
	let (tft_game_ids_sender, mut tft_game_ids_receiver) = tokio::sync::mpsc::channel(64);
	let tft_get_game_ids = async {
		let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
		for ((player, platform), ..) in tft_players_platforms_telegram_chats.iter().cycle() {
			interval.tick().await;
			let Ok(tft_game_ids) =
					api::riot::tft::get_last_game_ids(&tft_api, *platform, player).await
			else {
				continue
			};

			for tft_game_id in tft_game_ids {
				tft_game_ids_sender
					.send((tft_game_id, *platform))
					.await
					.unwrap_or_default();
			}
		}
	};

	// LOL game getter task
	let (lol_games_sender, mut lol_games_receiver) = tokio::sync::mpsc::unbounded_channel();
	let lol_get_games = async {
		let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
		while let Some((lol_game_id, riot_platform)) = lol_game_ids_receiver.recv().await {
			if db::riot::lol::contains_game(&db_pool, &lol_game_id, riot_platform)
				.await
				.unwrap_or(true)
			{
				continue;
			}

			interval.tick().await;
			let Ok(Some(lol_game)) =
					api::riot::lol::get_game(&lol_api, riot_platform, &lol_game_id).await
			else {
				continue
			};

			let lol_players_participants_telegram_chats = lol_game
				.info
				.participants
				.iter()
				.filter_map(|lol_participant| {
					lol_players_platforms_telegram_chats
						.iter()
						.find(|((lol_player, ..), ..)| lol_player.puuid == lol_participant.puuid)
						.map(|((lol_player, ..), telegram_chats)| {
							(lol_player, lol_participant, telegram_chats)
						})
				});

			let mut lol_players_participants_leagues_telegram_chats = Vec::default();
			for (lol_player, lol_participant, telegram_chats) in
				lol_players_participants_telegram_chats
			{
				interval.tick().await;
				let lol_leagues = api::riot::lol::get_leagues(&lol_api, riot_platform, lol_player)
					.await
					.unwrap_or_default();

				let lol_league = lol_leagues.into_iter().find(|lol_league| {
					api::riot::are_same_queue(&lol_league.queue_type, lol_game.info.queue_id)
				});
				lol_players_participants_leagues_telegram_chats.push((
					lol_player,
					lol_participant,
					lol_league,
					telegram_chats,
				));
			}

			if db::riot::lol::insert_game(
				&db_pool,
				&lol_game,
				riot_platform,
				&lol_players_participants_leagues_telegram_chats,
			)
			.await
			.is_err()
			{
				continue;
			}

			for telegram_chat in telegram_chats.iter().copied() {
				let lol_players_participants_leagues =
					lol_players_participants_leagues_telegram_chats
						.iter()
						.filter(|(.., telegram_chats)| telegram_chats.contains(&telegram_chat))
						.map(|(lol_player, lol_participant, lol_league, ..)| {
							(
								(*lol_player).clone(),
								(*lol_participant).clone(),
								lol_league.clone(),
							)
						})
						.collect::<Vec<_>>();
				lol_games_sender
					.send((
						telegram_chat,
						lol_game.clone(),
						riot_platform,
						lol_players_participants_leagues,
					))
					.unwrap_or_default();
			}
		}
	};

	// TFT game getter task
	let (tft_games_sender, mut tft_games_receiver) = tokio::sync::mpsc::unbounded_channel();
	let tft_get_games = async {
		let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
		while let Some((tft_game_id, riot_platform)) = tft_game_ids_receiver.recv().await {
			if db::riot::tft::contains_game(&db_pool, &tft_game_id, riot_platform)
				.await
				.unwrap_or(true)
			{
				continue;
			}

			interval.tick().await;
			let Ok(Some(tft_game)) =
					api::riot::tft::get_game(&tft_api, riot_platform, &tft_game_id).await
			else {
				continue
			};

			let tft_players_participants_telegram_chats = tft_game
				.info
				.participants
				.iter()
				.filter_map(|tft_participant| {
					tft_players_platforms_telegram_chats
						.iter()
						.find(|((tft_player, ..), ..)| tft_player.puuid == tft_participant.puuid)
						.map(|((tft_player, ..), telegram_chats)| {
							(tft_player, tft_participant, telegram_chats)
						})
				});

			let mut tft_players_participants_leagues_telegram_chats = Vec::default();
			for (tft_player, tft_participant, telegram_chats) in
				tft_players_participants_telegram_chats
			{
				interval.tick().await;
				let tft_leagues = api::riot::tft::get_leagues(&tft_api, riot_platform, tft_player)
					.await
					.unwrap_or_default();

				let tft_league = tft_leagues.into_iter().find(|tft_league| {
					api::riot::are_same_queue_id(&tft_league.queue_type, tft_game.info.queue_id)
				});
				tft_players_participants_leagues_telegram_chats.push((
					tft_player,
					tft_participant,
					tft_league,
					telegram_chats,
				));
			}

			if db::riot::tft::insert_game(
				&db_pool,
				&tft_game,
				riot_platform,
				&tft_players_participants_leagues_telegram_chats,
			)
			.await
			.is_err()
			{
				continue;
			}

			for telegram_chat in telegram_chats.iter().copied() {
				let tft_players_participants_leagues =
					tft_players_participants_leagues_telegram_chats
						.iter()
						.filter(|(.., telegram_chats)| telegram_chats.contains(&telegram_chat))
						.map(|(tft_player, tft_participant, tft_league, ..)| {
							(
								(*tft_player).clone(),
								(*tft_participant).clone(),
								tft_league.clone(),
							)
						})
						.collect::<Vec<_>>();
				tft_games_sender
					.send((
						telegram_chat,
						tft_game.clone(),
						riot_platform,
						tft_players_participants_leagues,
					))
					.unwrap_or_default();
			}
		}
	};

	let (messages_sender, mut messages_receiver) = tokio::sync::mpsc::unbounded_channel();

	// LOL message generator task
	let lol_generate_messages = async {
		while let Some((telegram_chat, lol_game, riot_platform, lol_players_participants_leagues)) =
			lol_games_receiver.recv().await
		{
			let messages = message::riot::lol::generate_message(
				&lol_game,
				riot_platform,
				&lol_players_participants_leagues,
				&config.riot_lol_message,
			);
			messages_sender
				.send((telegram_chat, messages))
				.unwrap_or_default();
		}
	};

	// TFT message generator task
	let tft_generate_messages = async {
		while let Some((telegram_chat, tft_game, riot_platform, tft_players_participants_leagues)) =
			tft_games_receiver.recv().await
		{
			let messages = message::riot::tft::generate_message(
				&tft_game,
				riot_platform,
				&tft_players_participants_leagues,
				&config.riot_tft_message,
			);
			messages_sender
				.send((telegram_chat, messages))
				.unwrap_or_default();
		}
	};

	// Telegram notifier task
	let telegram_notify = async {
		// Telegram API instance
		let telegram_api = api::telegram::Throttle::new_spawn(
			api::telegram::Api::new(config.telegram_api_key),
			api::telegram::Limits::default(),
		);

		while let Some((telegram_chat, messages)) = messages_receiver.recv().await {
			let mut messages_failed = Vec::default();
			for message in messages {
				if api::telegram::send_message(&telegram_api, telegram_chat, &message)
					.await
					.is_err()
				{
					messages_failed.push(message);
				}
			}
			if !messages_failed.is_empty() {
				messages_sender
					.send((telegram_chat, messages_failed))
					.unwrap_or_default();
			}
		}
	};

	// Run tasks
	tokio::join!(
		lol_get_game_ids,
		tft_get_game_ids,
		lol_get_games,
		tft_get_games,
		lol_generate_messages,
		tft_generate_messages,
		telegram_notify,
	);
}
