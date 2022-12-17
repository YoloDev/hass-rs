use crate::topics::TopicsConfig;
use async_trait::async_trait;
use hass_mqtt_provider::{MqttClient, MqttProvider, MqttProviderCreateError};

pub(crate) struct HassMqttConnection<T>
where
	T: MqttClient,
{
	pub(crate) topics: TopicsConfig,
	pub(crate) client: T,
}

#[async_trait(?Send)]
pub(crate) trait MqttProviderExt: MqttProvider {
	async fn create_client(
		options: &crate::HassMqttOptions,
	) -> Result<HassMqttConnection<Self::Client>, Self::Error> {
		let node_id = options.node_id.clone();
		let client_id = format!("{}_{}", options.application_name.slug(), options.node_id);
		let topics = TopicsConfig::new(
			options
				.private_prefix
				.as_deref()
				.unwrap_or_else(|| options.application_name.slug()),
			&*options.discovery_prefix,
			node_id.clone(),
		);
		let online_message = topics
			.online_message()
			.map_err(|e| Self::Error::create_message("online", e))?;
		let offline_message = topics
			.offline_message()
			.map_err(|e| Self::Error::create_message("offline", e))?;

		let client = Self::create(options, &client_id, online_message, offline_message).await?;
		Ok(HassMqttConnection { topics, client })
	}
}

#[async_trait(?Send)]
impl<T: MqttProvider> MqttProviderExt for T {}
