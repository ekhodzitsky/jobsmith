//! Salary lookup command.

use std::path::Path;

use tracing::info;

use crate::error::{JobsmithError, Result};
use crate::salary::lookup::SalaryLookup;

/// Run the salary lookup command.
pub fn run(company: &str, city: Option<&str>, json: bool, data_dir: &Path) -> Result<()> {
    let data_path = data_dir.join("salary_data.json");

    if !data_path.exists() {
        return Err(JobsmithError::Config(
            "salary_data.json not found. see documentation for setup instructions.".to_string(),
        ));
    }

    info!(company = %company, "looking up salary data");
    let lookup = SalaryLookup::from_file(&data_path)?;
    let results = lookup.search(company, city);

    if results.is_empty() {
        return Err(JobsmithError::SalaryNotFound {
            query: company.to_string(),
        });
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&results).map_err(JobsmithError::Json)?
        );
    } else {
        println!("\nFound {} match(es) for '{}':\n", results.len(), company);
        for entry in &results {
            println!("{}", "=".repeat(60));
            println!("  {}", entry.company);
            if let Some(city) = &entry.city {
                println!("  City: {}", city);
            }
            if let Some(categories) = &entry.categories {
                println!("  Categories: {}", categories);
            }
            println!();
        }
    }

    Ok(())
}
