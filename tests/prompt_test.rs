use jobsmith::workflow::prompts;

mod common;

#[test]
fn build_fit_evaluation_prompt_is_non_empty() {
    let profile = common::dummy_profile();
    let vacancy = common::dummy_vacancy_detail();
    let prompt = prompts::build_fit_evaluation_prompt(&profile, &vacancy);
    assert!(!prompt.is_empty());
}

#[test]
fn build_fit_evaluation_prompt_contains_expected_sections() {
    let profile = common::dummy_profile();
    let vacancy = common::dummy_vacancy_detail();
    let prompt = prompts::build_fit_evaluation_prompt(&profile, &vacancy);

    assert!(prompt.contains("Профиль кандидата"));
    assert!(prompt.contains("Вакансия"));
    assert!(prompt.contains("Инструкция"));
    assert!(prompt.contains("SCORE:"));
    assert!(prompt.contains("VERDICT:"));
    assert!(prompt.contains(&profile.name));
    assert!(prompt.contains(vacancy.base.employer_name()));
}

#[test]
fn build_cv_draft_prompt_is_non_empty() {
    let profile = common::dummy_profile();
    let vacancy = common::dummy_vacancy_detail();
    let prompt = prompts::build_cv_draft_prompt(&profile, &vacancy, "evaluation text");
    assert!(!prompt.is_empty());
}

#[test]
fn build_cv_draft_prompt_contains_expected_sections() {
    let profile = common::dummy_profile();
    let vacancy = common::dummy_vacancy_detail();
    let prompt = prompts::build_cv_draft_prompt(&profile, &vacancy, "evaluation text");

    assert!(prompt.contains("Контекст"));
    assert!(prompt.contains("Профиль кандидата"));
    assert!(prompt.contains("Вакансия"));
    assert!(prompt.contains("Инструкция"));
}

#[test]
fn build_cv_draft_prompt_requests_summary_only() {
    let profile = common::dummy_profile();
    let vacancy = common::dummy_vacancy_detail();
    let prompt = prompts::build_cv_draft_prompt(&profile, &vacancy, "evaluation text");

    // cv.typ renders Опыт работы/Навыки/Образование/Сертификаты/Языки from the
    // profile itself; if the prompt also requests them, every PDF duplicates
    // each section. The prompt must ask only for the summary block.
    for section in [
        "**Контактная информация**",
        "**Опыт работы**",
        "**Образование**",
        "**Сертификаты**",
        "**Языки**",
    ] {
        assert!(
            !prompt.contains(section),
            "prompt must not request profile section {section:?}"
        );
    }
    assert!(
        prompt.contains("Профессиональное резюме"),
        "prompt must request the summary block that fills {{{{SUMMARY}}}}"
    );
}

#[test]
fn build_cover_draft_prompt_is_non_empty() {
    let profile = common::dummy_profile();
    let vacancy = common::dummy_vacancy_detail();
    let prompt = prompts::build_cover_draft_prompt(&profile, &vacancy, "cv draft text");
    assert!(!prompt.is_empty());
}

#[test]
fn build_cover_draft_prompt_contains_expected_sections() {
    let profile = common::dummy_profile();
    let vacancy = common::dummy_vacancy_detail();
    let prompt = prompts::build_cover_draft_prompt(&profile, &vacancy, "cv draft text");

    assert!(prompt.contains("Профиль кандидата"));
    assert!(prompt.contains("Вакансия"));
    assert!(prompt.contains("Резюме"));
    assert!(prompt.contains("Инструкция"));
}

#[test]
fn build_reviewer_prompt_is_non_empty() {
    let vacancy = common::dummy_vacancy_detail();
    let prompt = prompts::build_reviewer_prompt(&common::dummy_profile(), &vacancy, "cv", "cover");
    assert!(!prompt.is_empty());
}

#[test]
fn build_reviewer_prompt_contains_expected_sections() {
    let vacancy = common::dummy_vacancy_detail();
    let prompt = prompts::build_reviewer_prompt(&common::dummy_profile(), &vacancy, "cv", "cover");

    assert!(prompt.contains("Вакансия"));
    assert!(prompt.contains("Резюме кандидата"));
    assert!(prompt.contains("Сопроводительное письмо"));
    assert!(prompt.contains("Инструкция"));
    assert!(prompt.contains("SCORES:"));
    assert!(prompt.contains("CRITIQUE:"));
}

#[test]
fn build_revision_prompt_is_non_empty() {
    let profile = common::dummy_profile();
    let prompt = prompts::build_revision_prompt(&profile, "cv", "cover", "review");
    assert!(!prompt.is_empty());
}

#[test]
fn build_revision_prompt_contains_expected_sections() {
    let profile = common::dummy_profile();
    let prompt = prompts::build_revision_prompt(&profile, "cv", "cover", "review");

    assert!(prompt.contains("Профиль кандидата"));
    assert!(prompt.contains("Черновик резюме"));
    assert!(prompt.contains("Черновик сопроводительного письма"));
    assert!(prompt.contains("Ревью"));
    assert!(prompt.contains("---CV---"));
    assert!(prompt.contains("---COVER---"));
}
