//! Apply command — run the full AI workflow for a vacancy.

use std::path::Path;

use tracing::{info, instrument};

use crate::cli::VacancySource;
use crate::error::{JobsmithError, Result};
use crate::habr::HabrClient;
use crate::hh::client::{extract_vacancy_id, HhClient};
use crate::hh::models::VacancyDetail;
use crate::profile::store::{ApplicationStatus, ProfileStore};
use crate::templates;
use crate::workflow::state::Stage;
use crate::workflow::{AcpClient, WorkflowEngine};

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
    source: VacancySource,
) -> Result<()> {
    let profile = store.load_profile().await?.ok_or_else(|| {
        JobsmithError::Config("profile not found. run `jobsmith setup` first".to_string())
    })?;

    // Extract the source-specific ID from the URL/raw input
    let id = match source {
        // numeric ids for both
        VacancySource::Hh | VacancySource::Habr => extract_vacancy_id(vacancy_id)?,
        VacancySource::Trudvsem => {
            let (cc, vid) = crate::trudvsem::api::extract_card_ids(vacancy_id)?;
            format!("{cc}/{vid}")
        }
    };
    info!(vacancy_id = %id, ?source, "fetching vacancy");

    let vacancy = fetch_vacancy(source, &id).await?;
    println!(
        "\n=== {} @ {} ===\n",
        vacancy.base.name,
        vacancy.base.employer_name()
    );

    let mut kimi_client = AcpClient::spawn().await?;
    let engine = WorkflowEngine::new();
    let stage = engine.start(vacancy.clone(), profile.clone());

    println!("Running application workflow...\n");
    let final_stage = engine.run(stage, &mut kimi_client, force).await?;

    let output_dir = templates::ensure_output_dir(data_dir).await?;

    let workflow_result = finalize_application(store, &id, force, &output_dir, final_stage).await;

    if let Err(e) = kimi_client.shutdown().await {
        tracing::warn!(error = %e, "kimi shutdown failed");
    }
    workflow_result
}

/// Fetch the vacancy detail from the selected job board.
async fn fetch_vacancy(source: VacancySource, id: &str) -> Result<VacancyDetail> {
    match source {
        VacancySource::Hh => HhClient::new()?.get_vacancy(id).await,
        VacancySource::Habr => HabrClient::new()?.get_vacancy(id).await,
        VacancySource::Trudvsem => {
            let (cc, vid) = crate::trudvsem::api::extract_card_ids(id)?;
            crate::trudvsem::TrudvsemClient::new()?
                .get_vacancy(&cc, &vid)
                .await
        }
    }
}

/// Compile documents, transition to [`Stage::Done`] and record the application.
///
/// Split from `run` so the post-workflow persistence logic is testable
/// without a live kimi/HH/typst environment.
async fn finalize_application(
    store: &ProfileStore,
    id: &str,
    force: bool,
    output_dir: &Path,
    final_stage: Stage,
) -> Result<()> {
    {
        let done = match final_stage {
            Stage::CompilePdf {
                vacancy,
                profile,
                evaluation,
                final_cv,
                final_cover,
            } => {
                let cv_typst =
                    templates::generate_cv_typst(&profile, &vacancy, &final_cv, output_dir).await?;
                let cover_typst =
                    templates::generate_cover_typst(&profile, &vacancy, &final_cover, output_dir)
                        .await?;

                let employer_safe = templates::sanitize_filename(vacancy.base.employer_name());
                let role_safe = templates::sanitize_filename(&vacancy.base.name);
                let id_safe = templates::sanitize_filename(id);
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
                        id,
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hh::models::{Vacancy, VacancyDetail};
    use crate::profile::model::Profile;
    use crate::workflow::state::FitScore;

    fn vacancy_detail(id: &str) -> VacancyDetail {
        VacancyDetail {
            base: Vacancy {
                id: id.to_string(),
                name: "Rust Dev".to_string(),
                description: None,
                salary: None,
                employer: None,
                area: None,
                vacancy_type: None,
                experience: None,
                schedule: None,
                employment: None,
                key_skills: None,
                published_at: None,
                created_at: None,
                alternate_url: None,
                apply_alternate_url: None,
                address: None,
                snippet: None,
                working_days: None,
                working_time_intervals: None,
                working_time_modes: None,
                accept_temporary: None,
                professional_roles: None,
            },
            contacts: None,
            department: None,
            branded_description: None,
            hidden: None,
            response_letter_required: None,
            relocation: None,
            request_id: None,
        }
    }

    fn done_stage(vacancy_id: &str, score: i32) -> Stage {
        Stage::Done {
            vacancy: vacancy_detail(vacancy_id),
            profile: Profile::default(),
            evaluation: FitScore::new(score).unwrap(),
            cv_pdf_path: "/tmp/cv.pdf".to_string(),
            cover_pdf_path: "/tmp/cover.pdf".to_string(),
        }
    }

    #[tokio::test]
    async fn finalize_records_done_stage() {
        let store = ProfileStore::open_in_memory().await.unwrap();
        let out = std::env::temp_dir();

        finalize_application(&store, "556", false, &out, done_stage("556", 77))
            .await
            .unwrap();

        let apps = store.list_applications().await.unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].vacancy_id, "556");
        assert_eq!(apps[0].status, ApplicationStatus::Draft);
        assert_eq!(apps[0].fit_score, Some(77));
        assert_eq!(apps[0].cv_path.as_deref(), Some("/tmp/cv.pdf"));
        assert_eq!(apps[0].cover_path.as_deref(), Some("/tmp/cover.pdf"));
    }

    #[tokio::test]
    async fn finalize_stores_null_score_for_forced_runs() {
        let store = ProfileStore::open_in_memory().await.unwrap();
        let out = std::env::temp_dir();

        finalize_application(&store, "555", true, &out, done_stage("555", 100))
            .await
            .unwrap();

        let apps = store.list_applications().await.unwrap();
        assert_eq!(
            apps[0].fit_score, None,
            "forced run must not store a synthetic score"
        );
    }

    #[tokio::test]
    async fn finalize_rejects_non_terminal_stage() {
        let store = ProfileStore::open_in_memory().await.unwrap();
        let stage = Stage::EvaluateFit {
            vacancy: vacancy_detail("1"),
            profile: Profile::default(),
        };

        let err = finalize_application(&store, "1", false, &std::env::temp_dir(), stage)
            .await
            .unwrap_err();

        assert!(
            matches!(err, JobsmithError::InvalidWorkflowState { .. }),
            "{err:?}"
        );
        assert!(store.list_applications().await.unwrap().is_empty());
    }
}
