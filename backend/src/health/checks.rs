use crate::AppState;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DependencyStatus {
    pub postgres: bool,
    pub redis: bool,
    pub nats: bool,
}

impl DependencyStatus {
    pub fn all_healthy(&self) -> bool {
        self.postgres && self.redis && self.nats
    }
}

pub async fn check_dependencies(state: &AppState) -> DependencyStatus {
    let postgres = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    let redis = match state.redis.get_multiplexed_tokio_connection().await {
        Ok(mut conn) => redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .is_ok(),
        Err(_) => false,
    };

    let nats = matches!(
        state.nats.connection_state(),
        async_nats::connection::State::Connected
    );

    DependencyStatus {
        postgres,
        redis,
        nats,
    }
}
