use chrono::Local;
use csv::Reader;
use std::fs::File;
use std::io::BufReader;

#[tokio::test]
async fn test_top_100_active_completeness() {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    export_top_100_active(&pool).await.unwrap();

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("output/top_100_active_{}.csv", timestamp);
    let file = File::open(&filename).unwrap();
    let reader = BufReader::new(file);
    let mut csv_reader = Reader::from_reader(reader);

    let headers = csv_reader.headers().unwrap();
    assert_eq!(headers.len(), 13);

    let mut count = 0;
    for result in csv_reader.records() {
        let record = result.unwrap();
        assert_eq!(record[8], "true");
        count += 1;
    }

    assert_eq!(count, 100);
}
