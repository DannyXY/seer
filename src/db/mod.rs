use redis::Client as RedisClient;
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::config::Settings;

#[derive(Clone)]
pub struct Infrastructure {
    pub postgres: Option<PgPool>,
    pub redis: Option<RedisClient>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InfrastructureStatus {
    pub postgres_configured: bool,
    pub redis_configured: bool,
}

impl Infrastructure {
    pub fn from_settings(settings: &Settings) -> anyhow::Result<Self> {
        let postgres = settings
            .database_url
            .as_ref()
            .map(|url| PgPoolOptions::new().max_connections(5).connect_lazy(url))
            .transpose()?;

        let redis = settings
            .redis_url
            .as_ref()
            .map(|url| RedisClient::open(url.as_str()))
            .transpose()?;

        Ok(Self { postgres, redis })
    }

    pub fn status(&self) -> InfrastructureStatus {
        InfrastructureStatus {
            postgres_configured: self.postgres.is_some(),
            redis_configured: self.redis.is_some(),
        }
    }
}
