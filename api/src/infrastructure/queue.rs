use std::sync::Arc;

use apalis::layers::WorkerBuilderExt;
use apalis::layers::retry::RetryPolicy;
use apalis::prelude::*;
use apalis_redis::RedisStorage;
use redis::AsyncCommands;
use tokio::time::{Duration, sleep, timeout};

use crate::application::jobs::{
    FastJob, MediumJob, SlowJob, handle_fast_job, handle_medium_job, handle_slow_job,
};
use crate::shared::app_state::AppState;

pub struct QueueStorages {
    pub fast: RedisStorage<FastJob>,
    pub medium: RedisStorage<MediumJob>,
    pub slow: RedisStorage<SlowJob>,
}

pub async fn create_storages(redis_url: &str) -> QueueStorages {
    let redis_conn = create_redis_connection_with_retry(redis_url).await;
    QueueStorages {
        fast: RedisStorage::new(redis_conn.clone()),
        medium: RedisStorage::new(redis_conn.clone()),
        slow: RedisStorage::new(redis_conn),
    }
}

pub fn build_monitor(app_state: Arc<AppState>) -> Monitor {
    // Spawn apalis workers (in-process).
    // Worker names must be unique per boot: apalis-redis rejects registering a
    // worker name that is still marked active in Redis (keep-alive threshold),
    // so reusing fixed names makes workers exit immediately on quick restarts
    // and also prevents running workers on multiple API replicas.
    let worker_id = format!("{:08x}", rand::random::<u32>());
    let fast_worker_name = format!("fast-job-worker-{worker_id}");
    let medium_worker_name = format!("medium-job-worker-{worker_id}");
    let slow_worker_name = format!("slow-job-worker-{worker_id}");

    Monitor::new()
        .register({
            let app_state = app_state.clone();
            let fast_worker_name = fast_worker_name.clone();
            move |_| {
                // One or more workers pulling from Redis
                WorkerBuilder::new(fast_worker_name.clone())
                    .backend(app_state.fast_jobs.as_ref().clone())
                    .catch_panic()
                    .retry(RetryPolicy::retries(7))
                    .enable_tracing()
                    .concurrency(6) // Adjust concurrency as needed
                    .data(app_state.clone())
                    .build(handle_fast_job)
            }
        })
        .register({
            let app_state = app_state.clone();
            let medium_worker_name = medium_worker_name.clone();
            move |_| {
                WorkerBuilder::new(medium_worker_name.clone())
                    .backend(app_state.medium_jobs.as_ref().clone())
                    .catch_panic()
                    .retry(RetryPolicy::retries(7))
                    .enable_tracing()
                    .concurrency(4)
                    .data(app_state.clone())
                    .build(handle_medium_job)
            }
        })
        .register({
            let app_state = app_state.clone();
            let slow_worker_name = slow_worker_name.clone();
            move |_| {
                WorkerBuilder::new(slow_worker_name.clone())
                    .backend(app_state.slow_jobs.as_ref().clone())
                    .catch_panic()
                    .retry(RetryPolicy::retries(7))
                    .enable_tracing()
                    .concurrency(2)
                    .data(app_state.clone())
                    .build(handle_slow_job)
            }
        })
        .on_event(|_, e| {
            if matches!(e, Event::HeartBeat) {
                tracing::debug!("{e}");
            } else {
                tracing::info!("{e}");
            }
        })
    // Wait 5 seconds after shutdown is triggered to allow any incomplete jobs to complete
    // .shutdown_timeout(Duration::from_secs(5))
}

async fn check_redis(redis_url: &str) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = timeout(
        Duration::from_secs(3),
        client.get_multiplexed_async_connection(),
    )
    .await??;

    timeout(Duration::from_secs(3), conn.ping::<String>()).await??;

    Ok(())
}

async fn create_redis_connection_with_retry(redis_url: &str) -> apalis_redis::ConnectionManager {
    let mut attempt = 1;

    loop {
        match check_redis(redis_url).await {
            Ok(()) => match apalis_redis::connect(redis_url.to_string()).await {
                Ok(conn) => {
                    tracing::info!(attempts = attempt, "Redis connection created");
                    return conn;
                }
                Err(err) => {
                    tracing::warn!(
                        attempt,
                        error = %err,
                        "Redis connection unavailable; retrying in 5 seconds"
                    );
                }
            },
            Err(err) => {
                tracing::warn!(
                    attempt,
                    error = %err,
                    "Redis not reachable; retrying in 5 seconds"
                );
            }
        }

        sleep(Duration::from_secs(5)).await;
        attempt += 1;
    }
}
