//! Apply command — run the full AI workflow for a vacancy.

use std::path::Path;

use tracing::{info, instrument};

use crate::error::{JobsmithError, Result};
use crate::hh::client::{extract_vacancy_id, HhClient};
use crate::profile::store::{ApplicationStatus, ProfileStore};
use crate::templates;
use crate::workflow::state::Stage;
use crate::workflow::{KimiClient, WorkflowEngine};

/// Run the apply command.
///
/// `data_dir` is the same root the caller opened the database in, so
/// generated documents land next to it (honours `--data-dir`).
#[instrument(skip(store, vacancy_id, data_dir))]
pub async fn run(
    store: &ProfileStore,
    vacancy_id: &str,
    force: bool,
    data_dir: &Path,
) -> Result<()> {
    let profile = store.load_profile().await?.ok_or_else(|| {
        JobsmithError::Config("profile not found. run `jobsmith setup` first".to_string())
    })?;

    let client = HhClient::new()?;

    // Extract ID from URL if needed
    let id = extract_vacancy_id(vacancy_id)?;
    info!(vacancy_id = %id, "fetching vacancy");

    let vacancy = client.get_vacancy(&id).await?;
    println!(
        "\n=== {} @ {} ===\n",
        vacancy.base.name,
        vacancy.base.employer_name()
    );

    let mut kimi_client = KimiClient::spawn().await?;
    let engine = WorkflowEngine::new();
    let stage = engine.start(vacancy.clone(), profile.clone());

    println!("Running application workflow...\n");
    let final_stage = engine.run(stage, &mut kimi_client, force).await?;

    let output_dir = templates::ensure_output_dir(data_dir).await?;

    let workflow_result = async {
        let done = match final_stage {
            Stage::CompilePdf {
                vacancy,
                profile,
                evaluation,
                final_cv,
                final_cover,
            } => {
                let cv_typst =
                    templates::generate_cv_typst(&profile, &vacancy, &final_cv, &output_dir)
                        .await?;
                let cover_typst =
                    templates::generate_cover_typst(&profile, &vacancy, &final_cover, &output_dir)
                        .await?;

                let employer_safe = templates::sanitize_filename(vacancy.base.employer_name());
                let role_safe = templates::sanitize_filename(&vacancy.base.name);
                let id_safe = templates::sanitize_filename(&id);
                let cv_pdf = output_dir.join(format!("cv_{}_{}.pdf", employer_safe, id_safe));
                let cover_pdf =
                    output_dir.join(format!("cover_{}_{}.pdf", employer_safe, role_safe));

                templates::compile_typst(&cv_typst, &cv_pdf).await?;
                templates::compile_typst(&cover_typst, &cover_pdf).await?;

                // Rebuild-and-transition through the FSM, mirroring the
                // WorkflowEngine::run idiom, so Done is the single state
                // that gets recorded below.
                Stage::CompilePdf {
                    vacancy,
                    profile,
                    evaluation,
                    final_cv,
                    final_cover,
                }
                .into_done(
                    cv_pdf.display().to_string(),
                    cover_pdf.display().to_string(),
                )?
            }
            done @ Stage::Done { .. } => done,
            other => {
                return Err(JobsmithError::InvalidWorkflowState {
                    expected: "compile_pdf or done".to_string(),
                    actual: other.name().to_string(),
                });
            }
        };

        match done {
            Stage::Done {
                vacancy,
                evaluation,
                cv_pdf_path,
                cover_pdf_path,
                ..
            } => {
                let app_id = store
                    .record_application(
                        &id,
                        Some(&vacancy.base.name),
                        Some(vacancy.base.employer_name()),
                    )
                    .await?;
                // forced runs skip evaluation: a synthetic 100 would corrupt history
                let recorded_score = if force {
                    None
                } else {
                    Some(evaluation.score())
                };
                store
                    .update_application_status(
                        app_id,
                        ApplicationStatus::Draft,
                        recorded_score,
                        Some(&cv_pdf_path),
                        Some(&cover_pdf_path),
                    )
                    .await?;

                println!("\n✓ Application recorded (id: {}).", app_id);
                println!("CV: {}", cv_pdf_path);
                println!("Cover letter: {}", cover_pdf_path);
            }
            // into_done only produces Done; defensive arm, never a panic
            other => {
                return Err(JobsmithError::InvalidWorkflowState {
                    expected: "done".to_string(),
                    actual: other.name().to_string(),
                });
            }
        }

        println!("Output directory: {}", output_dir.display());
        Ok::<(), JobsmithError>(())
    }
    .await;

    if let Err(e) = kimi_client.shutdown().await {
        tracing::warn!(error = %e, "kimi shutdown failed");
    }
    workflow_result
}
