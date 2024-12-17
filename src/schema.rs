use std::path::{Path, PathBuf};

use colored::{ColoredString, Colorize};
use log::{error, info};
use schemars::{generate::SchemaSettings, JsonSchema, SchemaGenerator};

use crate::{config::Config, module::{repository::RepositoryConfig, ModuleConfig}, output::table::{table, Column}, registry::cache::default_cache_dir};

pub const DEFAULT_DIR: &str = "/schemas";

/// Finds the current schema directory ([`crate::registry::cache::default_cache_dir`][`DEFAULT_DIR`])
pub fn schema_dir() -> String {
    default_cache_dir() + DEFAULT_DIR
}

/// Write all schema json files to the specified directory
pub fn write_schemas(directory: &String) {
    let path = match create_schema_dir(&directory) {
        Ok(path) => path,
        Err(err) => {
            error!("Failed to write schemas: {err}");
            return;
        },
    };

    let columns = [
        Column::new("Information").ellipse(),
        Column::new("Location").force(),
    ];

    let mut rows: Vec<[ColoredString; 2]> = vec![];
    let mut errors = vec![];

    let path_str = path.to_str().unwrap_or_default();

    info!("{}", "Schemas:".underline().bold());

    let mut generator = SchemaSettings::draft07().into_generator();

    match write_schema::<Config>(&mut generator, &path, "config.json") {
        Ok(_) => rows.push([ "Schema for pusta configuration".into(), format!("{path_str}/config.json").dimmed() ]),
        Err(err) => errors.push(format!("Failed to write config schema: {err}")),
    }
    match write_schema::<ModuleConfig>(&mut generator, &path,"module.json") {
        Ok(_) => rows.push([ "Schema for module configurations".into(), format!("{path_str}/module.json").dimmed() ]),
        Err(err) => errors.push(format!("Failed to write module schema: {err}")),
    }
    match write_schema::<RepositoryConfig>(&mut generator, &path,"repository.json") {
        Ok(_) => rows.push([ "Schema for repository configurations".into(), format!("{path_str}/repository.json").dimmed() ]),
        Err(err) => errors.push(format!("Failed to write repository schema: {err}")),
    }

    table(columns, rows, "  ");

    if !errors.is_empty() {
        println!();
        info!("{}", "Errors:".underline().bold());
        errors.iter().for_each(|err| {
            info!("- {}", err.bright_red())
        });
    }
}

/// Create and ensure the integrity of the schema directory
fn create_schema_dir(directory: &String) -> Result<PathBuf, String> {
    let dir_path = Path::new(&directory);
    if dir_path.exists() && !dir_path.is_dir() {
        return Err(format!("{directory} exists and is not a directory"))
    } else if !dir_path.exists() {
        std::fs::create_dir_all(dir_path).map_err(|err| format!("Failed to create directory {directory}: {err}"))?
    }

    Ok(dir_path.to_path_buf())
}

fn write_schema<T: JsonSchema>(generator: &mut SchemaGenerator, path: &PathBuf, name: &str) -> Result<(), String> {
    let schema = generator.root_schema_for::<T>();
    let schema_str = serde_json::to_string_pretty(&schema)
        .map_err(|err| format!("Failed to convert schema to json: {err}"))?;

    let path = path
        .join(name);
    let path_str = path.to_str().unwrap_or_default().to_owned();

    std::fs::write(path, schema_str)
        .map_err(|err| format!("Failed to write schema to {}: {err}", path_str))
}