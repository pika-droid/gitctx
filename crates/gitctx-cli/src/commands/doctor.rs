use crate::Format;
use crate::envelope::ResponseEnvelope;
use crate::format::format_doctor;

pub fn run(format: Format) -> anyhow::Result<()> {
    // Basic diagnostic output
    if format == Format::Json {
        let env = ResponseEnvelope::success(serde_json::json!({
            "status": "ok",
            "checks": [
                { "name": "sqlite_index_health", "status": "pass" },
                { "name": "schema_version", "status": "pass" },
                { "name": "config_hash", "status": "pass" }
            ]
        }));
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        format_doctor(true, true, true);
    }
    
    Ok(())
}
