//! Workflow state machine.
//!
//! Implements the drafter-reviewer pipeline as a typed state machine.
//! Each stage carries the data needed for the next transition.

use tracing::{info, instrument};

use crate::error::{JobsmithError, Result};
use crate::hh::models::VacancyDetail;
use crate::profile::model::Profile;

/// A fit evaluation score (0-100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FitScore(pub i32);

impl FitScore {
    /// Minimum score to proceed with application.
    pub const MIN_ACCEPTABLE: i32 = 60;

    /// Check if the score is acceptable.
    pub fn is_acceptable(&self) -> bool {
        self.0 >= Self::MIN_ACCEPTABLE
    }
}

/// The workflow state machine.
///
/// Each variant represents a stage in the application pipeline.
/// The state carries all data produced so far, making invalid transitions
/// unrepresentable at the type level.
#[derive(Debug, Clone)]
pub enum Stage {
    /// Initial state: have a vacancy and profile, need to evaluate fit.
    EvaluateFit {
        vacancy: VacancyDetail,
        profile: Profile,
    },
    /// Fit evaluated, need to draft CV and cover letter.
    DraftCv {
        vacancy: VacancyDetail,
        profile: Profile,
        evaluation: FitScore,
        evaluation_text: String,
    },
    /// Drafts created, need reviewer critique.
    Review {
        vacancy: VacancyDetail,
        profile: Profile,
        evaluation: FitScore,
        cv_draft: String,
        cover_draft: String,
    },
    /// Review received, need to revise.
    Revise {
        vacancy: VacancyDetail,
        profile: Profile,
        evaluation: FitScore,
        cv_draft: String,
        cover_draft: String,
        review: String,
    },
    /// Revised, need to compile PDF.
    CompilePdf {
        vacancy: VacancyDetail,
        profile: Profile,
        evaluation: FitScore,
        final_cv: String,
        final_cover: String,
    },
    /// Final state: PDFs compiled.
    Done {
        vacancy: VacancyDetail,
        profile: Profile,
        evaluation: FitScore,
        cv_pdf_path: String,
        cover_pdf_path: String,
    },
}

impl Stage {
    /// Human-readable name of the stage.
    pub fn name(&self) -> &'static str {
        match self {
            Stage::EvaluateFit { .. } => "evaluate_fit",
            Stage::DraftCv { .. } => "draft_cv",
            Stage::Review { .. } => "review",
            Stage::Revise { .. } => "revise",
            Stage::CompilePdf { .. } => "compile_pdf",
            Stage::Done { .. } => "done",
        }
    }

    /// Transition from EvaluateFit to DraftCv.
    pub fn into_draft_cv(
        self,
        evaluation: FitScore,
        evaluation_text: String,
    ) -> Result<Self> {
        match self {
            Stage::EvaluateFit { vacancy, profile } => {
                if !evaluation.is_acceptable() {
                    return Err(JobsmithError::FitScoreTooLow {
                        score: evaluation.0,
                        min: FitScore::MIN_ACCEPTABLE,
                    });
                }
                Ok(Stage::DraftCv {
                    vacancy,
                    profile,
                    evaluation,
                    evaluation_text,
                })
            }
            other => Err(JobsmithError::InvalidWorkflowState {
                expected: "evaluate_fit".to_string(),
                actual: other.name().to_string(),
            }),
        }
    }

    /// Transition from DraftCv to Review.
    pub fn into_review(self, cv_draft: String, cover_draft: String) -> Result<Self> {
        match self {
            Stage::DraftCv {
                vacancy,
                profile,
                evaluation,
                ..
            } => Ok(Stage::Review {
                vacancy,
                profile,
                evaluation,
                cv_draft,
                cover_draft,
            }),
            other => Err(JobsmithError::InvalidWorkflowState {
                expected: "draft_cv".to_string(),
                actual: other.name().to_string(),
            }),
        }
    }

    /// Transition from Review to Revise.
    pub fn into_revise(self, review: String) -> Result<Self> {
        match self {
            Stage::Review {
                vacancy,
                profile,
                evaluation,
                cv_draft,
                cover_draft,
            } => Ok(Stage::Revise {
                vacancy,
                profile,
                evaluation,
                cv_draft,
                cover_draft,
                review,
            }),
            other => Err(JobsmithError::InvalidWorkflowState {
                expected: "review".to_string(),
                actual: other.name().to_string(),
            }),
        }
    }

    /// Transition from Revise to CompilePdf.
    pub fn into_compile_pdf(self, final_cv: String, final_cover: String) -> Result<Self> {
        match self {
            Stage::Revise {
                vacancy,
                profile,
                evaluation,
                ..
            } => Ok(Stage::CompilePdf {
                vacancy,
                profile,
                evaluation,
                final_cv,
                final_cover,
            }),
            other => Err(JobsmithError::InvalidWorkflowState {
                expected: "revise".to_string(),
                actual: other.name().to_string(),
            }),
        }
    }

    /// Transition from CompilePdf to Done.
    pub fn into_done(self, cv_pdf_path: String, cover_pdf_path: String) -> Result<Self> {
        match self {
            Stage::CompilePdf {
                vacancy,
                profile,
                evaluation,
                ..
            } => Ok(Stage::Done {
                vacancy,
                profile,
                evaluation,
                cv_pdf_path,
                cover_pdf_path,
            }),
            other => Err(JobsmithError::InvalidWorkflowState {
                expected: "compile_pdf".to_string(),
                actual: other.name().to_string(),
            }),
        }
    }
}

/// Workflow engine that runs the state machine.
///
/// In production, this integrates with Kimi Code via `kimi-wire`.
/// For now, it provides the structure and prompt builders.
#[derive(Debug)]
pub struct WorkflowEngine;

impl WorkflowEngine {
    /// Create a new workflow engine.
    pub fn new() -> Self {
        Self
    }

    /// Start a workflow for a vacancy.
    #[instrument(skip(self, vacancy, profile))]
    pub fn start(&self, vacancy: VacancyDetail, profile: Profile) -> Stage {
        info!(vacancy_id = %vacancy.base.id, "starting workflow");
        Stage::EvaluateFit { vacancy, profile }
    }

    /// Run the full workflow via Kimi Code.
    ///
    /// Each stage sends a prompt through the wire client and advances the state
    /// machine with the parsed response. Stops at `CompilePdf` so the caller
    /// can compile PDFs and transition to `Done`.
    pub async fn run(
        &self,
        mut stage: Stage,
        kimi_client: &mut crate::workflow::kimi::KimiClient,
    ) -> Result<Stage> {
        loop {
            stage = match stage {
                Stage::EvaluateFit { vacancy, profile } => {
                    let (score, evaluation_text) =
                        kimi_client.evaluate_fit(&profile, &vacancy).await?;
                    if !score.is_acceptable() {
                        return Err(JobsmithError::FitScoreTooLow {
                            score: score.0,
                            min: FitScore::MIN_ACCEPTABLE,
                        });
                    }
                    Stage::DraftCv {
                        vacancy,
                        profile,
                        evaluation: score,
                        evaluation_text,
                    }
                }
                Stage::DraftCv {
                    vacancy,
                    profile,
                    evaluation,
                    evaluation_text,
                } => {
                    let (cv_draft, cover_draft) =
                        kimi_client.draft_cv(&profile, &vacancy, &evaluation_text).await?;
                    Stage::Review {
                        vacancy,
                        profile,
                        evaluation,
                        cv_draft,
                        cover_draft,
                    }
                }
                Stage::Review {
                    vacancy,
                    profile,
                    evaluation,
                    cv_draft,
                    cover_draft,
                } => {
                    let review = kimi_client
                        .review(&profile, &vacancy, &cv_draft, &cover_draft)
                        .await?;
                    Stage::Revise {
                        vacancy,
                        profile,
                        evaluation,
                        cv_draft,
                        cover_draft,
                        review,
                    }
                }
                Stage::Revise {
                    vacancy,
                    profile,
                    evaluation,
                    cv_draft,
                    cover_draft,
                    review,
                } => {
                    let (final_cv, final_cover) = kimi_client
                        .revise(&profile, &cv_draft, &cover_draft, &review)
                        .await?;
                    Stage::CompilePdf {
                        vacancy,
                        profile,
                        evaluation,
                        final_cv,
                        final_cover,
                    }
                }
                Stage::CompilePdf { .. } => {
                    info!("workflow reached compile_pdf stage");
                    return Ok(stage);
                }
                Stage::Done { .. } => {
                    info!("workflow complete");
                    return Ok(stage);
                }
            };
        }
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}
