use clap::Subcommand;
use crate::models::{Dimension, Template};
use crate::config::validate_dimensions;
use crate::files;
use std::io::Read;
use std::path::Path;

#[derive(Subcommand)]
pub enum DimensionsCommands {
    /// Get dimensions for a month or the template
    Get {
        /// Year
        #[arg(long, required_unless_present = "template")]
        year: Option<i32>,
        /// Month (1-12)
        #[arg(long, required_unless_present = "template")]
        month: Option<u32>,
        /// Get template dimensions instead of monthly
        #[arg(long)]
        template: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Set dimensions for a month or the template (reads from stdin)
    Set {
        /// Year
        #[arg(long, required_unless_present = "template")]
        year: Option<i32>,
        /// Month (1-12)
        #[arg(long, required_unless_present = "template")]
        month: Option<u32>,
        /// Set template dimensions instead of monthly
        #[arg(long)]
        template: bool,
        /// Input is JSON instead of YAML
        #[arg(long)]
        json: bool,
    },
}

pub fn handle_dimensions(cmd: DimensionsCommands, root: &Path) -> Result<(), String> {
    match cmd {
        DimensionsCommands::Get { year, month, template, json } => {
            let dims: Vec<Dimension> = if template {
                files::read_dimensions_template(root)?.dimensions
            } else {
                let y = year.unwrap();
                let m = month.unwrap();
                let source_line = format!("# source: {}/{:02}/dimensions.yaml", y, m);
                let dims = files::resolve_month_dimensions(root, y, m)?;
                if !json {
                    println!("{}", source_line);
                }
                dims
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&dims).map_err(|e| e.to_string())?);
            } else {
                println!("{}", yaml_serde::to_string(&dims).map_err(|e| e.to_string())?);
            }
            Ok(())
        }
        DimensionsCommands::Set { year, month, template, json } => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input).map_err(|e| e.to_string())?;
            let dims: Vec<Dimension> = if json {
                serde_json::from_str(&input).map_err(|e| format!("Invalid JSON: {}", e))?
            } else {
                yaml_serde::from_str(&input).map_err(|e| format!("Invalid YAML: {}", e))?
            };
            // validate_dimensions returns Vec<ConfigErrorDetail> — empty Vec means valid
            let errors = validate_dimensions(&dims);
            if !errors.is_empty() {
                let messages: Vec<String> = errors.iter()
                    .map(|e| format!("[{}] {}", e.kind, e.message))
                    .collect();
                return Err(messages.join("\n"));
            }
            if template {
                let tmpl = Template { dimensions: dims };
                let path = files::dimensions_template_path(root);
                let yaml = yaml_serde::to_string(&tmpl).map_err(|e| e.to_string())?;
                let tmp = path.with_extension("tmp");
                std::fs::write(&tmp, &yaml).map_err(|e| e.to_string())?;
                std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
            } else {
                files::write_dimensions_file(root, year.unwrap(), month.unwrap(), &dims)?;
            }
            Ok(())
        }
    }
}
