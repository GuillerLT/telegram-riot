use super::{
	riot::{
		lol::{Game, League, Participant, Player},
		Platform, Rank, Tier,
	},
	Result, SqlitePool,
};

const PRODUCT: &str = "Lol";

pub async fn create_tables(pool: &SqlitePool) -> Result<()> {
	super::create_tables(pool, PRODUCT).await
}

pub async fn contains_game(pool: &SqlitePool, game_id: &str, platform: Platform) -> Result<bool> {
	super::contains_game(pool, PRODUCT, game_id, platform).await
}

pub async fn insert_game<T>(
	pool: &SqlitePool,
	game: &Game,
	platform: Platform,
	players_participants_leagues_: &[(&Player, &Participant, Option<League>, T)],
) -> Result<()> {
	super::insert_game(
		pool,
		PRODUCT,
		&game.metadata.match_id,
		platform,
		i32::from(game.info.queue_id.0),
		game.info.game_start_timestamp,
		&players_participants_leagues_
			.iter()
			.map(|(player, participant, league, ..)| {
				(
					player.puuid.as_str(),
					i32::from(participant.win),
					league.as_ref().map(|league| {
						(
							league.tier.unwrap_or(Tier::UNRANKED),
							league.rank.unwrap_or(Rank::I),
							league.league_points,
						)
					}),
				)
			})
			.collect::<Vec<_>>(),
	)
	.await
}

pub async fn insert_players<T>(
	pool: &SqlitePool,
	players_platforms_: &[((Player, Platform), T)],
) -> Result<()> {
	super::insert_players(
		pool,
		PRODUCT,
		players_platforms_
			.iter()
			.map(|((player, ..), ..)| (player.puuid.as_str(), player.name.as_str()))
			.collect::<Vec<_>>()
			.as_slice(),
	)
	.await
}
