use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingSettings {
    pub level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsSettings {
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitSettings {
    pub burst_size: u32,
    pub replenish_per_second: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CircuitBreakerSettings {
    pub failure_threshold: u32,
    pub cooldown_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OtelSettings {
    pub enabled: bool,
    pub otlp_endpoint: String,
    pub service_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub logging: LoggingSettings,
    pub cors: CorsSettings,
    pub rate_limit: RateLimitSettings,
    pub circuit_breaker: CircuitBreakerSettings,
    pub otel: OtelSettings,
}

impl Settings {
    /// Merges `config/base.toml` -> `config/{APP_ENV}.toml` -> `APP__*` env overrides.
    /// APP_ENV defaults to "development". Env overrides use double-underscore nesting,
    /// e.g. `APP__RATE_LIMIT__BURST_SIZE=50`.
    pub fn load() -> Result<Self, config::ConfigError> {
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

        let settings = config::Config::builder()
            .add_source(config::File::with_name("backend/config/base").required(false))
            .add_source(config::File::with_name("config/base").required(false))
            .add_source(
                config::File::with_name(&format!("backend/config/{}", app_env)).required(false),
            )
            .add_source(config::File::with_name(&format!("config/{}", app_env)).required(false))
            .add_source(config::Environment::with_prefix("APP").separator("__"))
            .build()?;

        settings.try_deserialize()
    }
}
