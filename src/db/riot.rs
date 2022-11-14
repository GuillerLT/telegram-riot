pub mod lol;
pub mod tft;

use super::{
	api::riot,
	api::riot::{Platform, Rank, Tier},
	Result, SqlitePool,
};

async fn create_tables(pool: &SqlitePool, product: &str) -> Result<()> {
	let mut transaction = pool.begin().await?;

	sqlx::query(
		&format!("CREATE TABLE IF NOT EXISTS Riot{product}Players (Id CHAR(78), Name VARCHAR(32), PRIMARY KEY (Id))"),
	)
	.execute(&mut transaction)
	.await
	.map_err(|err| {
		tracing::error!(product, error = err.to_string(), "Error creating DB table (Players)");
		err
	})?;

	sqlx::query(
		&format!("CREATE TABLE IF NOT EXISTS Riot{product}Games (Id CHAR(15), Platform CHAR(4), Queue CHAR(15), Date DATETIME, PRIMARY KEY (Id, Platform))"),
	)
	.execute(&mut transaction)
	.await
	.map_err(|err| {
		tracing::error!(product, error = err.to_string(), "Error creating DB table (Games)");
		err
	})?;

	sqlx::query(
		&format!("CREATE TABLE IF NOT EXISTS Riot{product}GameResults (GameId CHAR(15), Platform CHAR(4), PlayerId CHAR(78), Result TINYINT, LeagueTier TINYINT, LeagueRank TINYINT, LeaguePoints TINYINT, FOREIGN KEY (GameId, Platform) REFERENCES Riot{product}Games(Id, Platform) ON UPDATE CASCADE ON DELETE RESTRICT, FOREIGN KEY (PlayerId) REFERENCES Riot{product}Players(Id) ON UPDATE CASCADE ON DELETE RESTRICT, PRIMARY KEY (GameId, Platform, PlayerId))"),
	)
	.execute(&mut transaction)
	.await
	.map_err(|err| {
		tracing::error!(product, error = err.to_string(), "Error creating DB table (GameResults)");
		err
	})?;

	transaction.commit().await
}

async fn contains_game(
	pool: &SqlitePool,
	product: &str,
	game_id: &str,
	platform: Platform,
) -> Result<bool> {
	sqlx::query_scalar(&format!(
		"SELECT COUNT(*) FROM Riot{product}Games WHERE Id = ? AND Platform = ?"
	))
	.bind(game_id)
	.bind(platform.to_string())
	.fetch_one(pool)
	.await
	.map_err(|err| {
		tracing::error!(
			product,
			platform = platform.as_region_str(),
			game_id,
			error = err.to_string(),
			"Error reading DB"
		);
		err
	})
	.map(|result: i64| result > 0)
}

async fn insert_game(
	pool: &SqlitePool,
	product: &str,
	game_id: &str,
	platform: Platform,
	queue: i32,
	date: i64,
	player_ids_results_leagues: &[(&str, i32, Option<(Tier, Rank, i32)>)],
) -> Result<()> {
	let mut transaction = pool.begin().await?;

	let platform_string = platform.to_string();

	sqlx::query(&format!(
		"INSERT INTO Riot{product}Games (Id, Platform, Queue, Date) VALUES(?, ?, ?, ?)"
	))
	.bind(game_id)
	.bind(&platform_string)
	.bind(queue)
	.bind(
		chrono::NaiveDateTime::from_timestamp_opt(
			date / 1000,
			(date % 1000).try_into().unwrap_or_default(),
		)
		.map(|date| date.format("%Y-%m-%dT%H:%M:%S").to_string()),
	)
	.execute(&mut transaction)
	.await
	.map_err(|err| {
		tracing::error!(
			product,
			platform = platform.as_region_str(),
			game_id,
			error = err.to_string(),
			"Error writing DB (Games)"
		);
		err
	})?;

	if !player_ids_results_leagues.is_empty() {
		sqlx::query_builder::QueryBuilder::new(format!(
			"INSERT INTO Riot{product}GameResults (GameId, Platform, PlayerId, Result, LeagueTier, LeagueRank, LeaguePoints) "
		))
		.push_values(player_ids_results_leagues, |mut value, (player_id, result, league)| {
			let (tier, rank, points) = match league {
				Some((tier, rank, points)) => (Some(*tier), Some(*rank), Some(*points)),
				None => (None, None, None),
			};
			value
				.push_bind(game_id)
				.push_bind(&platform_string)
				.push_bind(*player_id)
				.push_bind(*result)
				.push_bind(tier.map(u8::from))
				.push_bind(rank.map(u8::from))
				.push_bind(points);
		})
		.build()
		.execute(&mut transaction)
		.await
		.map_err(|err| {
			tracing::error!(
				product,
				platform = platform.as_region_str(),
				game_id,
				error = err.to_string(),
				"Error writing DB (GameResults)"
			);
			err
		})?;
	}

	transaction.commit().await
}

async fn insert_players(
	pool: &SqlitePool,
	product: &str,
	player_ids_names: &[(&str, &str)],
) -> Result<()> {
	let mut transaction = pool.begin().await?;

	for (id, name) in player_ids_names {
		sqlx::query(&format!(
			"UPDATE OR IGNORE Riot{product}Players SET Name=? WHERE Id=?"
		))
		.bind(name)
		.bind(id)
		.execute(&mut transaction)
		.await
		.map_err(|err| {
			tracing::error!(
				product,
				error = err.to_string(),
				"Error writing DB (Players)"
			);
			err
		})?;
	}

	if !player_ids_names.is_empty() {
		sqlx::query_builder::QueryBuilder::new(format!(
			"INSERT OR IGNORE INTO Riot{product}Players (Id, Name) "
		))
		.push_values(player_ids_names, |mut value, (id, name)| {
			value.push_bind(id).push_bind(name);
		})
		.build()
		.execute(&mut transaction)
		.await
		.map_err(|err| {
			tracing::error!(
				product,
				error = err.to_string(),
				"Error writing DB (Players)"
			);
			err
		})?;
	}

	transaction.commit().await
}
