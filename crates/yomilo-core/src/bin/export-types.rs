use std::{error::Error, path::PathBuf};
use ts_rs::{Config, TS};
use yomilo_core::{
    InputState, PreparationInputs, RenderSegment, RequiredInputs, SegmentBatch,
    json::{CanonicalCheckResult, JsonApiError},
};

// TODO(native-types): ネイティブ描画へ文書モデルを渡すためのDart・Kotlinの型の生成方法は未定。
fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("Usage: export-types <output-directory>")?;
    let config = Config::new()
        .with_out_dir(output)
        .with_large_int("number")
        .with_import_extension(Some("js".to_owned()));
    SegmentBatch::export_all(&config)?;
    InputState::export_all(&config)?;
    RenderSegment::export_all(&config)?;
    PreparationInputs::export_all(&config)?;
    RequiredInputs::export_all(&config)?;
    JsonApiError::export_all(&config)?;
    CanonicalCheckResult::export_all(&config)?;
    Ok(())
}
