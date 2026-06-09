//! Apply command — run the full AI workflow for a vacancy.

use std::path::PathBuf;

use tracing::{info, instrument};

use crate::error::{JobsmithError, Result};
use crate::hh::client::{extract_vacancy_id, HhClient};
use crate::profile::store::{ApplicationStatus, ProfileStore};
use crate::templates;
use crate::workflow::{KimiClient, WorkflowEngine};
use crate::workflow::state::Stage;

/// Run the apply command.
#[instrument(skip(store, vacancy_id))]
pub async fn run(store: &ProfileStore, vacancy_id: &str, force: bool) -> Result<()> {
    let profile = store
        .load_profile().await?
        .ok_or_else(|| JobsmithError::Config("profile not found. run `jobsmith setup` first".to_string()))?;

    let client = HhClient::new()?;

    // Extract ID from URL if needed
    let id = extract_vacancy_id(vacancy_id)?;
    info!(vacancy_id = %id, "fetching vacancy");

    let vacancy = client.get_vacancy(&id).await?;
    println!("\n=== {} @ {} ===\n", vacancy.base.name, vacancy.base.employer_name());

    let mut kimi_client = KimiClient::spawn().await?;
    let engine = WorkflowEngine::new();
    let stage = engine.start(vacancy.clone(), profile.clone());

    println!("Running application workflow...\n");
    let final_stage = engine.run(stage, &mut kimi_client, force).await?;

    let output_dir = get_data_dir()?.join("output");
    templates::ensure_output_dir(&output_dir).await?;

    let workflow_result = async {
        match final_stage {
            Stage::CompilePdf {
                vacancy,
                profile,
                evaluation,
                final_cv,
                final_cover,
            } => {
                let cv_typst = templates::generate_cv_typst(
                    &profile,
                    &vacancy,
                    &final_cv,
                    &output_dir,
                )
                .await?;
                let cover_typst = templates::generate_cover_typst(
                    &profile,
                    &vacancy,
                    &final_cover,
                    &output_dir,
                )
                .await?;

                let employer_safe = templates::sanitize_filename(vacancy.base.employer_name());
                let role_safe = templates::sanitize_filename(&vacancy.base.name);
                let id_safe = templates::sanitize_filename(&id);
                let cv_pdf = output_dir.join(format!("cv_{}_{}.pdf", employer_safe, id_safe));
                let cover_pdf = output_dir.join(format!("cover_{}_{}.pdf", employer_safe, role_safe));

                templates::compile_typst(&cv_typst, &cv_pdf).await?;
                templates::compile_typst(&cover_typst, &cover_pdf).await?;

                let app_id = store.record_application(
                    &id,
                    Some(&vacancy.base.name),
                    Some(vacancy.base.employer_name()),
                ).await?;
                store.update_application_status(
                    app_id,
                    ApplicationStatus::Draft,
                    Some(evaluation.score()),
                    cv_pdf.to_str(),
                    cover_pdf.to_str(),
                ).await?;

                println!("\n✓ Application recorded (id: {}).", app_id);
                println!("CV: {}", cv_pdf.display());
                println!("Cover letter: {}", cover_pdf.display());
            }
            Stage::Done {
                vacancy,
                evaluation,
                cv_pdf_path,
                cover_pdf_path,
                ..
            } => {
                let app_id = store.record_application(
                    &id,
                    Some(&vacancy.base.name),
                    Some(vacancy.base.employer_name()),
                ).await?;
                store.update_application_status(
                    app_id,
                    ApplicationStatus::Draft,
                    Some(evaluation.score()),
                    Some(&cv_pdf_path),
                    Some(&cover_pdf_path),
                ).await?;

                println!("\n✓ Application recorded (id: {}).", app_id);
                println!("CV: {}", cv_pdf_path);
                println!("Cover letter: {}", cover_pdf_path);
            }
            other => {
                return Err(JobsmithError::InvalidWorkflowState {
                    expected: "compile_pdf or done".to_string(),
                    actual: other.name().to_string(),
                });
            }
        }

        println!("Output directory: {}", output_dir.display());
        Ok::<(), JobsmithError>(())
    }.await;

    if let Err(e) = kimi_client.shutdown().await {
        tracing::warn!(error = %e, "kimi shutdown failed");
    }
    workflow_result
}

fn get_data_dir() -> Result<PathBuf> {
    dirs::data_dir()
        .map(|d| d.join("jobsmith"))
        .ok_or_else(|| JobsmithError::Config("could not determine data directory".to_string()))
}
