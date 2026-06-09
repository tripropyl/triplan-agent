use triplan_agent::config::RuntimePaths;

#[test]
fn user_owned_configuration_paths_stay_under_user_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_root = temp.path().join("home/.triplan-agent");
    let app_data = temp.path().join("app-data");
    let paths = RuntimePaths::new(user_root.clone(), app_data.clone());

    let user_config_paths = [
        paths.user_config_dir().to_path_buf(),
        paths.user_config_path(),
        paths.providers_path(),
        paths.mcp_path(),
        paths.shadowbox_path(),
        paths.agent_profiles_dir(),
        paths.user_skills_dir(),
        paths.user_prompts_dir(),
        paths.user_compact_prompts_dir(),
    ];

    assert_eq!(paths.user_config_dir(), user_root.join(".agents"));
    for path in user_config_paths {
        assert!(path.starts_with(&user_root), "{}", path.display());
        assert!(!path.starts_with(&app_data), "{}", path.display());
    }

    assert!(paths.conversation_history_dir().starts_with(&user_root));
    assert!(paths.checkpoint_database_path().starts_with(&app_data));
    assert!(paths.system_resources_dir().starts_with(&app_data));
}
