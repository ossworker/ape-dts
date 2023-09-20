use std::collections::{HashMap, HashSet};

use dt_common::config::config_enums::DbType;
use dt_common::utils::time_util::TimeUtil;
use futures::executor::block_on;

use crate::{test_config_util::TestConfigUtil, test_runner::rdb_test_runner::DST};

use super::{
    mongo_test_runner::MongoTestRunner, precheck_test_runner::PrecheckTestRunner,
    rdb_check_test_runner::RdbCheckTestRunner, rdb_kafka_rdb_test_runner::RdbKafkaRdbTestRunner,
    rdb_struct_test_runner::RdbStructTestRunner, rdb_test_runner::RdbTestRunner,
    redis_test_runner::RedisTestRunner,
};

pub struct TestBase {}

#[allow(dead_code)]
impl TestBase {
    pub async fn run_snapshot_test(test_dir: &str) {
        let runner = RdbTestRunner::new(test_dir).await.unwrap();
        runner.run_snapshot_test(true).await.unwrap();
    }

    pub async fn run_snapshot_test_and_check_dst_count(
        test_dir: &str,
        db_type: &DbType,
        dst_expected_counts: HashMap<&str, usize>,
    ) {
        let runner = RdbTestRunner::new(test_dir).await.unwrap();
        runner.run_snapshot_test(false).await.unwrap();

        let assert_dst_count = |db_tb: &(String, String), count: usize| {
            let dst_data = block_on(runner.fetch_data(db_tb, DST)).unwrap();
            println!(
                "check dst table {:?} record count, expect: {}",
                db_tb, count
            );
            assert_eq!(dst_data.len(), count);
        };

        for (db_tb, count) in dst_expected_counts {
            let db_tb = RdbTestRunner::parse_full_tb_name(db_tb, db_type);
            assert_dst_count(&db_tb, count);
        }
    }

    pub async fn run_cdc_test(test_dir: &str, start_millis: u64, parse_millis: u64) {
        let runner = RdbTestRunner::new(test_dir).await.unwrap();
        runner
            .run_cdc_test(start_millis, parse_millis)
            .await
            .unwrap();
        // runner.run_cdc_test_with_different_configs(start_millis, parse_millis))
        //     .unwrap();
    }

    pub async fn run_ddl_test(test_dir: &str, start_millis: u64, parse_millis: u64) {
        let runner = RdbTestRunner::new(test_dir).await.unwrap();
        runner
            .run_ddl_test(start_millis, parse_millis)
            .await
            .unwrap();
    }

    pub async fn run_cycle_cdc_test(
        test_dir: &str,
        start_millis: u64,
        parse_millis: u64,
        transaction_database: &str,
        expect_num_map: HashMap<String, u8>,
    ) {
        let sub_paths = TestConfigUtil::get_absolute_sub_dir(test_dir);
        let mut handlers: Vec<tokio::task::JoinHandle<()>> = vec![];
        let mut runner_map: HashMap<String, RdbTestRunner> = HashMap::new();

        // init all ddls
        for sub_path in &sub_paths {
            let runner = RdbTestRunner::new_internal(
                format!("{}/{}", test_dir, sub_path.1).as_str(),
                TestConfigUtil::REPLACE_PARAM,
                sub_path.1.as_str(),
            )
            .await
            .unwrap();

            runner.initialization_ddl().await.unwrap();

            runner_map.insert(sub_path.1.to_owned(), runner);
        }

        // start task
        for sub_path in &sub_paths {
            let runner = runner_map.get(sub_path.1.as_str()).unwrap();
            handlers.push(runner.base.spawn_task().await.unwrap());
        }
        TimeUtil::sleep_millis(start_millis).await;

        // init all datas
        for sub_path in &sub_paths {
            let runner = runner_map.get(sub_path.1.as_str()).unwrap();
            runner.initialization_data().await.unwrap();
        }
        TimeUtil::sleep_millis(parse_millis).await;

        // do check
        for sub_path in &sub_paths {
            let runner = runner_map.get(sub_path.1.as_str()).unwrap();
            let transaction_full_name = format!("{}.{}", transaction_database, sub_path.1);

            let expect_num = if expect_num_map.contains_key(sub_path.1.as_str()) {
                Some(expect_num_map.get(sub_path.1.as_str()).unwrap().clone())
            } else {
                None
            };

            runner
                .run_cycle_cdc_data_check(
                    String::from(transaction_database),
                    transaction_full_name,
                    expect_num,
                )
                .await
                .unwrap();
        }

        for handler in handlers {
            handler.abort();
            while !handler.is_finished() {
                TimeUtil::sleep_millis(1).await;
            }
        }
    }

    pub async fn run_check_test(test_dir: &str) {
        let runner = RdbCheckTestRunner::new(test_dir).await.unwrap();
        runner.run_check_test().await.unwrap();
    }

    pub async fn run_review_test(test_dir: &str) {
        let runner = RdbCheckTestRunner::new(test_dir).await.unwrap();
        runner.run_review_test().await.unwrap();
    }

    pub async fn run_revise_test(test_dir: &str) {
        let runner = RdbCheckTestRunner::new(test_dir).await.unwrap();
        runner.run_revise_test().await.unwrap();
    }

    pub async fn run_mongo_snapshot_test(test_dir: &str) {
        let runner = MongoTestRunner::new(test_dir).await.unwrap();
        runner.run_snapshot_test(true).await.unwrap();
    }

    pub async fn run_mongo_snapshot_test_and_check_dst_count(
        test_dir: &str,
        dst_expected_counts: HashMap<(&str, &str), usize>,
    ) {
        let runner = MongoTestRunner::new(test_dir).await.unwrap();
        runner.run_snapshot_test(false).await.unwrap();

        let assert_dst_count = |db: &str, tb: &str, count: usize| {
            let dst_data = block_on(runner.fetch_data(db, tb, DST));
            assert_eq!(dst_data.len(), count);
        };

        for ((db, tb), count) in dst_expected_counts.iter() {
            assert_dst_count(db, tb, *count);
        }
    }

    pub async fn run_mongo_cdc_test(test_dir: &str, start_millis: u64, parse_millis: u64) {
        let runner = MongoTestRunner::new(test_dir).await.unwrap();
        runner
            .run_cdc_test(start_millis, parse_millis)
            .await
            .unwrap();
    }

    pub async fn run_mongo_cdc_resume_test(test_dir: &str, start_millis: u64, parse_millis: u64) {
        let runner = MongoTestRunner::new(test_dir).await.unwrap();
        runner
            .run_cdc_resume_test(start_millis, parse_millis)
            .await
            .unwrap();
    }

    pub async fn run_redis_snapshot_test(test_dir: &str) {
        let mut runner = RedisTestRunner::new_default(test_dir).await.unwrap();
        runner.run_snapshot_test().await.unwrap();
    }

    pub async fn run_redis_rejson_snapshot_test(test_dir: &str) {
        let mut runner = RedisTestRunner::new(test_dir, vec![' '], vec![('\'', '\'')])
            .await
            .unwrap();
        runner.run_snapshot_test().await.unwrap();
    }

    pub async fn run_redis_redisearch_snapshot_test(test_dir: &str) {
        let mut runner = RedisTestRunner::new(test_dir, vec![' '], vec![('\'', '\'')])
            .await
            .unwrap();
        runner.run_snapshot_test().await.unwrap();
    }

    pub async fn run_redis_cdc_test(test_dir: &str, start_millis: u64, parse_millis: u64) {
        let mut runner = RedisTestRunner::new_default(test_dir).await.unwrap();
        runner
            .run_cdc_test(start_millis, parse_millis)
            .await
            .unwrap();
    }

    pub async fn run_redis_rejson_cdc_test(test_dir: &str, start_millis: u64, parse_millis: u64) {
        let mut runner = RedisTestRunner::new(test_dir, vec![' '], vec![('\'', '\'')])
            .await
            .unwrap();
        runner
            .run_cdc_test(start_millis, parse_millis)
            .await
            .unwrap();
    }

    pub async fn run_mysql_struct_test(test_dir: &str) {
        let mut runner = RdbStructTestRunner::new(test_dir).await.unwrap();
        runner.run_mysql_struct_test().await.unwrap();

        runner.base.execute_clean_sqls().await.unwrap();
    }

    pub async fn run_pg_struct_test(test_dir: &str) {
        let mut runner = RdbStructTestRunner::new(test_dir).await.unwrap();
        runner.run_pg_struct_test().await.unwrap();

        runner.base.execute_clean_sqls().await.unwrap();
    }

    pub async fn run_precheck_test(
        test_dir: &str,
        ignore_check_items: &HashSet<String>,
        src_expected_results: &HashMap<String, bool>,
        dst_expected_results: &HashMap<String, bool>,
    ) {
        let runner = PrecheckTestRunner::new(test_dir).await.unwrap();
        runner
            .run_check(
                ignore_check_items,
                src_expected_results,
                dst_expected_results,
            )
            .await
            .unwrap();
    }

    pub async fn run_rdb_kafka_rdb_cdc_test(test_dir: &str, start_millis: u64, parse_millis: u64) {
        let runner = RdbKafkaRdbTestRunner::new(test_dir).await.unwrap();
        runner
            .run_cdc_test(start_millis, parse_millis)
            .await
            .unwrap();
    }

    pub async fn run_rdb_kafka_rdb_snapshot_test(
        test_dir: &str,
        start_millis: u64,
        parse_millis: u64,
    ) {
        let runner = RdbKafkaRdbTestRunner::new(test_dir).await.unwrap();
        runner
            .run_snapshot_test(start_millis, parse_millis)
            .await
            .unwrap();
    }
}
