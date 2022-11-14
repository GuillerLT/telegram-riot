pub mod lol;
pub mod tft;

const LAST_COUNT: i32 = 100;

pub use riven::{
	consts::{Division as Rank, GameMode, PlatformRoute as Platform, Queue, QueueType, Tier},
	Result, RiotApi as Api,
};

pub fn are_same_queue(queue_type: &QueueType, queue: Queue) -> bool {
	match (queue_type, queue) {
		// LOL
		(QueueType::RANKED_SOLO_5x5, Queue::SUMMONERS_RIFT_5V5_RANKED_SOLO)
		| (QueueType::RANKED_FLEX_SR, Queue::SUMMONERS_RIFT_5V5_RANKED_FLEX)
		// TFT
		| (QueueType::RANKED_TFT, Queue::CONVERGENCE_RANKED_TEAMFIGHT_TACTICS)
		| (QueueType::RANKED_TFT_TURBO, Queue::CONVERGENCE_RANKED_TEAMFIGHT_TACTICS_HYPER_ROLL_)
		| (
			QueueType::RANKED_TFT_DOUBLE_UP,
			Queue::CONVERGENCE_RANKED_TEAMFIGHT_TACTICS_DOUBLE_UP_WORKSHOP_,
		) => true,
		// Fallback
		_ => false,
	}
}

pub fn are_same_queue_id(queue_type: &QueueType, queue: i32) -> bool {
	queue
		.try_into()
		.map(|queue| are_same_queue(queue_type, Queue(queue)))
		.unwrap_or(false)
}
