-- 1. Enable compression on the telemetry hypertable
ALTER TABLE telemetry SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'satellite_id'
);

-- 2. Add a compression policy to automatically compress chunks older than 2 hours
SELECT add_compression_policy('telemetry', INTERVAL '2 hours');
