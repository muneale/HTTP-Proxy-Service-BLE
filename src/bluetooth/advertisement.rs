use crate::{Config, Result};
use bluer::{
    Adapter, 
    adv::{AdvertisementHandle, Advertisement}
};
use crate::constants::SERVICE_UUID;

pub async fn create_advertisement(adapter: &Adapter, config: &Config) -> Result<AdvertisementHandle> {
    let le_advertisement = Advertisement {
        service_uuids: vec![*SERVICE_UUID].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some(config.name.clone()),
        ..Default::default()
    };

    let handle = adapter.advertise(le_advertisement).await?;
    Ok(handle)
}