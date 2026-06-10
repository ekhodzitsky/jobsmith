use jobsmith::profile::{ApplicationStatus, ProfileStore};

mod common;

#[tokio::test]
async fn profile_save_load_roundtrip() {
    let store = ProfileStore::open_in_memory().await.unwrap();
    let profile = common::dummy_profile();

    store.save_profile(&profile).await.unwrap();
    let loaded = store.load_profile().await.unwrap();

    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap(), profile);
}

#[test]
fn profile_validation_passes_with_required_fields() {
    let profile = common::dummy_profile();
    profile.validate().unwrap();
}

#[test]
fn profile_validation_fails_without_name() {
    let mut profile = common::dummy_profile();
    profile.name.clear();
    let err = profile.validate().unwrap_err();
    assert!(err.to_string().contains("name is required"));
}

#[test]
fn profile_validation_fails_without_phone() {
    let mut profile = common::dummy_profile();
    profile.phone.clear();
    let err = profile.validate().unwrap_err();
    assert!(err.to_string().contains("phone is required"));
}

#[test]
fn profile_validation_fails_without_email() {
    let mut profile = common::dummy_profile();
    profile.email.clear();
    let err = profile.validate().unwrap_err();
    assert!(err.to_string().contains("email is required"));
}

#[tokio::test]
async fn record_application_dedupes_by_vacancy_id() {
    let store = ProfileStore::open_in_memory().await.unwrap();

    let first = store
        .record_application("123", Some("Dev"), Some("Corp"))
        .await
        .unwrap();
    let second = store
        .record_application("123", Some("Dev v2"), Some("Corp"))
        .await
        .unwrap();

    assert_eq!(first, second, "re-applying must reuse the existing row");
    let apps = store.list_applications().await.unwrap();
    assert_eq!(apps.len(), 1, "no duplicate rows for the same vacancy");
    assert_eq!(apps[0].vacancy_name.as_deref(), Some("Dev v2"));
}

#[tokio::test]
async fn mark_applied_updates_status_and_rejects_unknown_id() {
    let store = ProfileStore::open_in_memory().await.unwrap();
    let id = store.record_application("v1", None, None).await.unwrap();

    store.mark_applied(id).await.unwrap();
    let apps = store.list_applications().await.unwrap();
    assert_eq!(apps[0].status, ApplicationStatus::Applied);

    let err = store.mark_applied(9999).await.unwrap_err();
    assert!(err.to_string().contains("not found"), "{err}");
}

#[tokio::test]
async fn application_record_crud() {
    let store = ProfileStore::open_in_memory().await.unwrap();

    let app_id = store
        .record_application("vac-1", Some("Rust Dev"), Some("Yandex"))
        .await
        .unwrap();
    assert_eq!(app_id, 1);

    store
        .update_application_status(
            app_id,
            ApplicationStatus::Applied,
            Some(85),
            Some("/tmp/cv.pdf"),
            Some("/tmp/cover.pdf"),
        )
        .await
        .unwrap();

    let apps = store.list_applications().await.unwrap();
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].vacancy_id, "vac-1");
    assert_eq!(apps[0].status, ApplicationStatus::Applied);
    assert_eq!(apps[0].fit_score, Some(85));
    assert_eq!(apps[0].cv_path, Some("/tmp/cv.pdf".to_string()));
    assert_eq!(apps[0].cover_path, Some("/tmp/cover.pdf".to_string()));
}
