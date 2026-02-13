mod harness;

#[test]
fn fixture_creates_unique_ids() {
    let f1 = harness::TestFixture::new();
    let f2 = harness::TestFixture::new();
    assert_ne!(f1.id, f2.id);
    assert_ne!(f1.container_name(), f2.container_name());
    assert_ne!(f1.session_name(), f2.session_name());
}

#[test]
fn fixture_names_use_test_prefix() {
    let f = harness::TestFixture::new();
    assert!(f.container_name().starts_with("dual-test-"));
    assert!(f.session_name().starts_with("dual-test-"));
}

#[test]
fn fixture_container_name_within_docker_limit() {
    let f = harness::TestFixture::new();
    // Docker container names limited to 63 characters
    assert!(f.container_name().len() <= 63);
}

#[test]
fn fixture_short_id_is_8_chars() {
    let f = harness::TestFixture::new();
    assert_eq!(f.short_id.len(), 8);
    assert!(f.id.starts_with(&f.short_id));
}

#[test]
fn fixture_temp_dir_created_and_cleaned() {
    let dir;
    {
        let mut f = harness::TestFixture::new();
        dir = f.temp_dir();
        assert!(dir.exists());
    }
    // After Drop, temp dir should be cleaned up
    assert!(!dir.exists());
}

#[test]
fn fixture_temp_subdir_created() {
    let mut f = harness::TestFixture::new();
    let parent = f.temp_dir();
    let sub = harness::TestFixture::temp_subdir(&parent, "workspace");
    assert!(sub.exists());
    assert!(sub.ends_with("workspace"));
}

#[test]
fn fixture_test_config_parses() {
    let mut f = harness::TestFixture::new();
    let workspace_root = f.temp_dir();
    let config = harness::TestFixture::test_config(
        &workspace_root,
        r#"
[[repos]]
name = "test-app"
url = "/tmp/test-repo"
branches = ["main"]
ports = [3000]
"#,
    );
    assert_eq!(config.repos.len(), 1);
    assert_eq!(config.repos[0].name, "test-app");
    assert_eq!(config.repos[0].ports, vec![3000]);
}

#[test]
fn fixture_registers_containers() {
    let mut f = harness::TestFixture::new();
    let name = f.container_name();
    f.register_container(name.clone());
    // Just verify it doesn't panic — actual cleanup tested with Docker
}

#[test]
fn fixture_registers_tmux_sessions() {
    let mut f = harness::TestFixture::new();
    let name = f.session_name();
    f.register_tmux_session(name.clone());
    // Just verify it doesn't panic — actual cleanup tested with tmux
}

#[test]
fn cleanup_sweep_runs_without_error() {
    // Verify cleanup_sweep doesn't panic when no test resources exist
    harness::cleanup_sweep();
}
