use std::path::PathBuf;

use jobsmith::profile::ProfileStore;

mod common;

fn temp_db_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    PathBuf::from(format!(
        "/tmp/jobsmith_test_profile_{}_{}.db",
        std::process::id(),
        n
    ))
}

#[tokio::test]
async fn profile_save_load_roundtrip() {
    let path = temp_db_path();
    let store = ProfileStore::open(&path).unwrap();
    let profile = common::dummy_profile();

    store.save_profile(&profile).unwrap();
    let loaded = store.load_profile().unwrap();

    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap(), profile);

    let _ = std::fs::remove_file(&path);
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
async fn application_record_crud() {
    let path = temp_db_path();
    let store = ProfileStore::open(&path).unwrap();

    let app_id = store
        .record_application("vac-1", Some("Rust Dev"), Some("Yandex"))
        .await
        .unwrap();
    assert_eq!(app_id, 1);

    store
        .update_application_status(
            app_id,
            "applied",
            Some(85),
            Some("/tmp/cv.pdf"),
            Some("/tmp/cover.pdf"),
        )
        .await
        .unwrap();

    let apps = store.list_applications().unwrap();
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].vacancy_id, "vac-1");
    assert_eq!(apps[0].status, "applied");
    assert_eq!(apps[0].fit_score, Some(85));
    assert_eq!(apps[0].cv_path, Some("/tmp/cv.pdf".to_string()));
    assert_eq!(apps[0].cover_path, Some("/tmp/cover.pdf".to_string()));

    let _ = std::fs::remove_file(&path);
}
