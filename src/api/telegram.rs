pub use teloxide::{
	adaptors::throttle::{Limits, Throttle},
	types::ChatId,
	Bot as Api,
};

use teloxide::{
	payloads::SendMessageSetters,
	requests::{Request, Requester},
	types::ParseMode,
};

pub async fn send_message<E: std::fmt::Display>(
	api: impl Requester<Err = E>,
	chat: ChatId,
	message: &str,
) -> Result<(), E> {
	api.send_message(chat, message)
		.parse_mode(ParseMode::Html)
		.disable_notification(true)
		.send()
		.await
		.map_err(|err| {
			tracing::error!(error = err.to_string(), "Error sending Telegram message");
			err
		})
		.map(|_| ())
}
