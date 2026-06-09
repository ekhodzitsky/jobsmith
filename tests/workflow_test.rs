use jobsmith::error::JobsmithError;
use jobsmith::workflow::state::{FitScore, Stage, WorkflowEngine};

mod common;

#[test]
fn stage_names() {
    let vacancy = common::dummy_vacancy_detail();
    let profile = common::dummy_profile();

    let stage = Stage::EvaluateFit {
        vacancy: vacancy.clone(),
        profile: profile.clone(),
    };
    assert_eq!(stage.name(), "evaluate_fit");

    let stage = stage
        .into_draft_cv(FitScore(75), "good fit".to_string())
        .unwrap();
    assert_eq!(stage.name(), "draft_cv");

    let stage = stage
        .into_review("cv draft".to_string(), "cover draft".to_string())
        .unwrap();
    assert_eq!(stage.name(), "review");

    let stage = stage.into_revise("review text".to_string()).unwrap();
    assert_eq!(stage.name(), "revise");

    let stage = stage
        .into_compile_pdf("final cv".to_string(), "final cover".to_string())
        .unwrap();
    assert_eq!(stage.name(), "compile_pdf");

    let stage = stage
        .into_done("cv.pdf".to_string(), "cover.pdf".to_string())
        .unwrap();
    assert_eq!(stage.name(), "done");
}

#[test]
fn valid_transitions() {
    let vacancy = common::dummy_vacancy_detail();
    let profile = common::dummy_profile();

    let stage = WorkflowEngine::new().start(vacancy.clone(), profile.clone());
    assert_eq!(stage.name(), "evaluate_fit");

    let stage = stage
        .into_draft_cv(FitScore(75), "good fit".to_string())
        .unwrap();
    let stage = stage
        .into_review("cv".to_string(), "cover".to_string())
        .unwrap();
    let stage = stage.into_revise("rev".to_string()).unwrap();
    let stage = stage
        .into_compile_pdf("final cv".to_string(), "final cover".to_string())
        .unwrap();
    let stage = stage
        .into_done("cv.pdf".to_string(), "cover.pdf".to_string())
        .unwrap();

    assert_eq!(stage.name(), "done");
}

#[test]
fn invalid_transition_evaluate_fit_to_review() {
    let vacancy = common::dummy_vacancy_detail();
    let profile = common::dummy_profile();

    let stage = Stage::EvaluateFit { vacancy, profile };
    let err = stage
        .into_review("cv".to_string(), "cover".to_string())
        .unwrap_err();

    assert!(matches!(err, JobsmithError::InvalidWorkflowState { .. }));
    assert!(err.to_string().contains("draft_cv"));
    assert!(err.to_string().contains("evaluate_fit"));
}

#[test]
fn invalid_transition_draft_cv_to_revise() {
    let vacancy = common::dummy_vacancy_detail();
    let profile = common::dummy_profile();

    let stage = Stage::DraftCv {
        vacancy,
        profile,
        evaluation: FitScore(75),
        evaluation_text: "ok".to_string(),
    };
    let err = stage.into_revise("rev".to_string()).unwrap_err();

    assert!(matches!(err, JobsmithError::InvalidWorkflowState { .. }));
    assert!(err.to_string().contains("review"));
}

#[test]
fn fit_score_is_acceptable() {
    assert!(FitScore(60).is_acceptable());
    assert!(FitScore(61).is_acceptable());
    assert!(FitScore(100).is_acceptable());
    assert!(!FitScore(59).is_acceptable());
    assert!(!FitScore(0).is_acceptable());
}

#[test]
fn unacceptable_fit_score_blocks_transition() {
    let vacancy = common::dummy_vacancy_detail();
    let profile = common::dummy_profile();

    let stage = Stage::EvaluateFit { vacancy, profile };
    let err = stage
        .into_draft_cv(FitScore(59), "poor fit".to_string())
        .unwrap_err();

    assert!(matches!(err, JobsmithError::FitScoreTooLow { .. }));
}
