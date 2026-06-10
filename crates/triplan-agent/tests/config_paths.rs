use triplan_agent::config::{init_user_data_at, RuntimePaths};

#[test]
fn user_owned_configuration_paths_stay_under_user_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_root = temp.path().join("home/.triplan-agent");
    let app_data = temp.path().join("app-data");
    let paths = RuntimePaths::new(user_root.clone(), app_data.clone());

    let user_owned_paths = [
        paths.user_config_dir().to_path_buf(),
        paths.user_config_path(),
        paths.providers_path(),
        paths.conversation_history_dir(),
    ];
    let agent_config_paths = [
        paths.agent_config_dir().to_path_buf(),
        paths.mcp_path(),
        paths.shadowbox_path(),
        paths.agent_profiles_dir(),
        paths.user_skills_dir(),
        paths.user_prompts_dir(),
        paths.user_compact_prompts_dir(),
    ];

    assert_eq!(paths.user_config_dir(), user_root);
    assert_eq!(paths.agent_config_dir(), user_root.join(".agents"));
    assert_eq!(paths.user_config_path(), user_root.join("config.toml"));
    assert_eq!(
        paths.legacy_user_config_path(),
        user_root.join(".agents/config.toml")
    );
    assert_eq!(paths.providers_path(), user_root.join("providers.toml"));
    assert_eq!(
        paths.legacy_providers_path(),
        user_root.join(".agents/providers.toml")
    );
    for path in user_owned_paths {
        assert!(path.starts_with(&user_root), "{}", path.display());
        assert!(!path.starts_with(&app_data), "{}", path.display());
    }
    for path in agent_config_paths {
        assert!(path.starts_with(&user_root), "{}", path.display());
        assert!(!path.starts_with(&app_data), "{}", path.display());
        assert!(
            path.starts_with(paths.agent_config_dir()),
            "{}",
            path.display()
        );
    }

    assert!(paths.checkpoint_database_path().starts_with(&app_data));
    assert!(paths.system_resources_dir().starts_with(&app_data));
}

#[tokio::test]
async fn legacy_global_config_is_migrated_to_user_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_root = temp.path().join("home/.triplan-agent");
    let app_data = temp.path().join("app-data");
    let paths = RuntimePaths::new(user_root.clone(), app_data);
    let legacy_user_config = r#"workspace_name = "legacy-workspace"
default_agent = "default"
default_provider = "local"
"#;
    let legacy_provider_config = r#"[providers.local]
base_url = "http://127.0.0.1:8080/v1"
api_key_env = "LOCAL_API_KEY"
"#;

    std::fs::create_dir_all(paths.user_agents_dir()).expect("legacy dir");
    std::fs::write(paths.legacy_user_config_path(), legacy_user_config).expect("legacy config");
    std::fs::write(paths.legacy_providers_path(), legacy_provider_config).expect("legacy config");

    init_user_data_at(&paths).await.expect("init user data");

    let migrated_user_config = std::fs::read_to_string(paths.user_config_path()).expect("config");
    assert_eq!(migrated_user_config, legacy_user_config);
    let migrated = std::fs::read_to_string(paths.providers_path()).expect("provider config");
    assert_eq!(migrated, legacy_provider_config);
}
