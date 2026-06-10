use jobsmith::templates;

mod common;

#[tokio::test]
async fn cv_typst_source_contains_each_section_exactly_once() {
    let profile = common::dummy_profile();
    let vacancy = common::dummy_vacancy_detail();
    // Summary-only AI draft, per the build_cv_draft_prompt contract:
    // profile sections come from the template, not from the model.
    let cv_content = "Опытный Rust-инженер с фокусом на надёжные backend-сервисы.";

    let output_dir =
        std::env::temp_dir().join(format!("jobsmith-templates-test-{}", std::process::id()));
    tokio::fs::create_dir_all(&output_dir).await.unwrap();

    let typ_path = templates::generate_cv_typst(&profile, &vacancy, cv_content, &output_dir)
        .await
        .unwrap();
    let source = tokio::fs::read_to_string(&typ_path).await.unwrap();
    // best-effort cleanup of the temp dir; assertion below is what matters
    let _ = tokio::fs::remove_dir_all(&output_dir).await;

    for section in [
        "== Профессиональное резюме",
        "== Опыт работы",
        "== Навыки",
        "== Образование",
        "== Сертификаты и курсы",
        "== Языки",
        "== Публикации и проекты",
    ] {
        assert_eq!(
            source.matches(section).count(),
            1,
            "section {section:?} must appear exactly once in the generated .typ"
        );
    }
}
