use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use tempfile::tempdir;

use rusty::processing::pipeline::{LoaderStageImpl, WriterStageImpl};
use rusty::processing::traits::{ProcessingContext, ProcessingConfig};
use rusty::processing::PipelineStage;

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use arrow::array::{Float64Array, Int32Array, StringArray};
use tokio::runtime::Runtime;

fn write_file(path: &PathBuf, content: &str) {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).unwrap(); }
    let mut f = File::create(path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.sync_all().unwrap();
}

fn run_pipeline_and_validate(survey: &str,
                             series_header: &str,
                             series_row: &str,
                             data_header: &str,
                             data_row: &str,
                             area_header: &str,
                             area_row: &str,
                             item_header: &str,
                             item_row: &str,
                             period_header: &str,
                             period_row: &str) {
    // Create isolated workspace
    let dir = tempdir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let sc = survey.to_uppercase();
    let base = PathBuf::from(format!("data/raw/bls/{}", sc));

    // Write series
    let series_path = base.join(format!("{}.series", survey.to_lowercase()));
    write_file(&series_path, &format!("{}\n{}\n", series_header, series_row));

    // Write data
    let data_dir = base.join("data");
    let data_path = data_dir.join(format!("{}.data.0", survey.to_lowercase()));
    write_file(&data_path, &format!("{}\n{}\n", data_header, data_row));

    // Write lookups
    let map_dir = base.join("map");
    write_file(&map_dir.join(format!("{}.area", survey.to_lowercase())), &format!("{}\n{}\n", area_header, area_row));
    write_file(&map_dir.join(format!("{}.item", survey.to_lowercase())), &format!("{}\n{}\n", item_header, item_row));
    write_file(&map_dir.join(format!("{}.period", survey.to_lowercase())), &format!("{}\n{}\n", period_header, period_row));

    // Build context and run loader
    let mut context = ProcessingContext::new(ProcessingConfig::default());
    context.input_paths = vec![
        format!("data/raw/bls/{sc}/*.series"),
        format!("data/raw/bls/{sc}/data/*"),
        format!("data/raw/bls/{sc}/map/*"),
    ];

    let mut loader = LoaderStageImpl::new();
    let rt = Runtime::new().unwrap();
    rt.block_on(loader.execute(&mut context)).expect("loader execute failed");

    // Run writer
    let mut writer = WriterStageImpl::new();
    rt.block_on(writer.execute(&mut context)).expect("writer execute failed");

    // Read the combined parquet
    let combined = PathBuf::from(format!("data/final/{}/combined.parquet", sc));
    assert!(combined.exists(), "combined parquet not created: {:?}", combined);

    let file = File::open(&combined).unwrap();
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
    let schema = builder.schema().clone();
    let mut reader = builder.build().unwrap();
    let batch = reader.next().unwrap().unwrap();

    // Ensure expected columns exist
    let expected_cols = vec![
        "series_id", "series_title", "survey_code", "year", "period_code", "period_name",
        "value", "area_code", "area_name", "item_code", "item_name"
    ];
    for col in &expected_cols {
        assert!(schema.fields().iter().any(|f| f.name() == col), "missing column {col}");
    }

    // Build index map
    let mut idx = std::collections::HashMap::new();
    for (i, f) in schema.fields().iter().enumerate() { idx.insert(f.name().to_string(), i); }

    // Helper to fetch string value
    let sval = |name: &str| -> String {
        let i = *idx.get(name).unwrap();
        let arr = batch.column(i).as_any().downcast_ref::<StringArray>().unwrap();
        arr.value(0).to_string()
    };
    let ival = |name: &str| -> i32 {
        let i = *idx.get(name).unwrap();
        let arr = batch.column(i).as_any().downcast_ref::<Int32Array>().unwrap();
        arr.value(0)
    };
    let fval = |name: &str| -> f64 {
        let i = *idx.get(name).unwrap();
        let arr = batch.column(i).as_any().downcast_ref::<Float64Array>().unwrap();
        arr.value(0)
    };

    // Validate values derived from our raw files
    assert_eq!(sval("series_id"), "TESTSERIES1");
    assert_eq!(sval("series_title"), "Test Series Title");
    assert_eq!(sval("survey_code"), sc);
    assert_eq!(ival("year"), 2024);
    assert_eq!(sval("period_code"), "M01");
    assert_eq!(sval("period_name"), "January");
    assert!((fval("value") - 1.23).abs() < 1e-9);
    assert_eq!(sval("area_code"), "A110");
    assert_eq!(sval("area_name"), "Area 110");
    assert_eq!(sval("item_code"), "711111");
    assert_eq!(sval("item_name"), "Item 711111");
}

#[test]
fn test_end_to_end_two_surveys_combined_parquet() {
    // AP survey minimal dataset
    run_pipeline_and_validate(
        "AP",
        "series_id\tseries_title\tarea_code\titem_code\tseasonal\tperiodicity_code\tbase_code\tbase_period",
        "TESTSERIES1\tTest Series Title\tA110\t711111\tU\tM\tB\t1982",
        "series_id\tyear\tperiod\tvalue",
        "TESTSERIES1\t2024\tM01\t1.23",
        "area_code\tarea_name",
        "A110\tArea 110",
        "item_code\titem_name",
        "711111\tItem 711111",
        "period\tperiod_abbr\tperiod_name",
        "M01\tJAN\tJanuary",
    );

    // WP survey minimal dataset
    run_pipeline_and_validate(
        "WP",
        "series_id\tseries_title\tarea_code\titem_code\tseasonal\tperiodicity_code\tbase_code\tbase_period",
        "TESTSERIES1\tTest Series Title\tA110\t711111\tU\tM\tB\t1982",
        "series_id\tyear\tperiod\tvalue",
        "TESTSERIES1\t2024\tM01\t1.23",
        "area_code\tarea_name",
        "A110\tArea 110",
        "item_code\titem_name",
        "711111\tItem 711111",
        "period\tperiod_abbr\tperiod_name",
        "M01\tJAN\tJanuary",
    );
}
