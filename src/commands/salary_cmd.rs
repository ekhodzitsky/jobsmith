//! Salary lookup command.

use std::path::Path;

use tracing::info;

use crate::error::{JobsmithError, Result};
use crate::hh::client::HhClient;
use crate::hh::models::SalaryStatisticsResponse;
use crate::salary::lookup::SalaryLookup;

/// Run the online salary statistics lookup (HH `/salary_statistics`).
pub async fn run_online(
    client: &HhClient,
    role: &str,
    area: &str,
    currency: &str,
    json: bool,
) -> Result<()> {
    info!(role, area, "fetching salary statistics from hh");
    let stats = client.get_salary_statistics(role, area, currency).await?;

    if stats.items.is_empty() {
        return Err(JobsmithError::SalaryNotFound {
            query: format!("role {role} in area {area}"),
        });
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&stats).map_err(JobsmithError::Json)?
        );
    } else {
        print!("{}", format_stats(&stats, currency));
    }

    Ok(())
}

fn format_stats(stats: &SalaryStatisticsResponse, currency: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "\nHH salary statistics ({} item(s), {currency}):\n\n",
        stats.items.len()
    ));
    for item in &stats.items {
        out.push_str(&format!("{}\n", "=".repeat(60)));
        out.push_str(&format!(
            "  {}\n",
            item.category.as_deref().unwrap_or("(без категории)")
        ));
        if let Some(salary) = &item.salary {
            let fmt = |v: Option<i32>| v.map(|n| n.to_string()).unwrap_or_else(|| "?".to_string());
            out.push_str(&format!(
                "  min {} | avg {} | max {}\n",
                fmt(salary.min),
                fmt(salary.avg),
                fmt(salary.max)
            ));
        }
        if let Some(p) = &item.percentiles {
            let fmt = |v: Option<i32>| v.map(|n| n.to_string()).unwrap_or_else(|| "—".to_string());
            out.push_str(&format!(
                "  p10 {} | p25 {} | p50 {} | p75 {} | p90 {}\n",
                fmt(p.p10),
                fmt(p.p25),
                fmt(p.p50),
                fmt(p.p75),
                fmt(p.p90)
            ));
        }
        out.push('\n');
    }
    out
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hh::models::{Percentiles, SalaryStatValue, SalaryStatisticsItem};

    #[test]
    fn format_stats_renders_category_values_and_percentiles() {
        let stats = SalaryStatisticsResponse {
            items: vec![SalaryStatisticsItem {
                category: Some("Разработчик".to_string()),
                salary: Some(SalaryStatValue {
                    min: Some(150_000),
                    max: Some(400_000),
                    avg: Some(250_000),
                }),
                percentiles: Some(Percentiles {
                    p10: None,
                    p25: Some(180_000),
                    p50: Some(250_000),
                    p75: Some(320_000),
                    p90: None,
                }),
            }],
        };

        let out = format_stats(&stats, "RUR");
        assert!(out.contains("Разработчик"), "{out}");
        assert!(
            out.contains("min 150000 | avg 250000 | max 400000"),
            "{out}"
        );
        assert!(out.contains("p25 180000"), "{out}");
        assert!(
            out.contains("p10 —"),
            "missing percentile must render as dash: {out}"
        );
        assert!(out.contains("RUR"), "{out}");
    }
}
