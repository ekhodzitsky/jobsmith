//! Workflow client trait for AI-powered pipeline stages.
//!
//! Abstracts over real (`KimiClient`) and mock implementations for testing.

use std::future::Future;

use crate::error::Result;
use crate::hh::models::VacancyDetail;
use crate::profile::model::Profile;
use crate::workflow::state::FitScore;

/// Trait for clients that can execute the application workflow pipeline.
pub trait WorkflowClient {
    /// Evaluate how well the profile fits the vacancy.
    fn evaluate_fit<'a>(
        &'a mut self,
        profile: &'a Profile,
        vacancy: &'a VacancyDetail,
    ) -> impl Future<Output = Result<(FitScore, String)>> + 'a;

    /// Draft CV and cover letter.
    fn draft_cv<'a>(
        &'a mut self,
        profile: &'a Profile,
        vacancy: &'a VacancyDetail,
        evaluation_text: &'a str,
    ) -> impl Future<Output = Result<(String, String)>> + 'a;

    /// Review the drafted documents.
    fn review<'a>(
        &'a mut self,
        profile: &'a Profile,
        vacancy: &'a VacancyDetail,
        cv_draft: &'a str,
        cover_draft: &'a str,
    ) -> impl Future<Output = Result<String>> + 'a;

    /// Revise documents based on review feedback.
    fn revise<'a>(
        &'a mut self,
        profile: &'a Profile,
        cv_draft: &'a str,
        cover_draft: &'a str,
        review: &'a str,
    ) -> impl Future<Output = Result<(String, String)>> + 'a;
}
