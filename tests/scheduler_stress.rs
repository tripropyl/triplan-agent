use std::collections::HashSet;
use std::sync::Arc;

use agent_ease::db::{connect_sqlite, migrate, TaskStore};
use agent_ease::runtime::SchedulerWorker;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scheduler_workers_drain_tasks_once_under_concurrency() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_url = format!(
        "sqlite://{}?mode=rwc",
        temp.path().join("stress.db").to_string_lossy()
    );
    let pool = connect_sqlite(&db_url).await.expect("pool");
    migrate(&pool).await.expect("migrate");
    let tasks = TaskStore::new(pool);

    let task_count = 300;
    for index in 0..task_count {
        tasks
            .enqueue(
                "stress_task",
                Some("stress-run"),
                10,
                serde_json::json!({"index": index}),
            )
            .await
            .expect("enqueue");
    }

    let seen = Arc::new(Mutex::new(HashSet::new()));
    let mut joins = Vec::new();
    for worker_index in 0..12 {
        let worker = SchedulerWorker::new(tasks.clone(), format!("worker-{worker_index}"));
        let seen = Arc::clone(&seen);
        joins.push(tokio::spawn(async move {
            worker
                .drain_available(|task| {
                    let seen = Arc::clone(&seen);
                    async move {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        let mut seen = seen.lock().await;
                        assert!(seen.insert(task.task_id), "task processed twice");
                        Ok(())
                    }
                })
                .await
        }));
    }

    let results = timeout(Duration::from_secs(20), async move {
        let mut completed = 0;
        for join in joins {
            completed += join.await.expect("join").expect("worker").completed;
        }
        completed
    })
    .await
    .expect("stress run timed out");

    assert_eq!(results, task_count);
    assert_eq!(seen.lock().await.len(), task_count as usize);
    assert_eq!(
        tasks.count_by_status("completed").await.expect("count"),
        task_count
    );
}
