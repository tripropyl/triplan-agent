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
        paths.mcp_path(),
        paths.shadowbox_path(),
        paths.agent_profiles_dir(),
        paths.user_prompts_dir(),
        paths.user_compact_prompts_dir(),
        paths.conversation_history_dir(),
    ];
    let agent_asset_paths = [
        paths.agent_assets_dir().to_path_buf(),
        paths.user_skills_dir(),
    ];

    assert_eq!(paths.user_config_dir(), user_root);
    assert_eq!(paths.agent_assets_dir(), user_root.join(".agents"));
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
    assert_eq!(paths.mcp_path(), user_root.join("mcp.toml"));
    assert_eq!(paths.legacy_mcp_path(), user_root.join(".agents/mcp.toml"));
    assert_eq!(paths.shadowbox_path(), user_root.join("shadowbox.toml"));
    assert_eq!(
        paths.legacy_shadowbox_path(),
        user_root.join(".agents/shadowbox.toml")
    );
    assert_eq!(paths.agent_profiles_dir(), user_root.join("agents"));
    assert_eq!(
        paths.legacy_agent_profiles_dir(),
        user_root.join(".agents/agents")
    );
    assert_eq!(paths.user_prompts_dir(), user_root.join("prompts"));
    assert_eq!(
        paths.legacy_user_prompts_dir(),
        user_root.join(".agents/prompts")
    );
    for path in user_owned_paths {
        assert!(path.starts_with(&user_root), "{}", path.display());
        assert!(!path.starts_with(&app_data), "{}", path.display());
        assert!(
            !path.starts_with(paths.agent_assets_dir()),
            "{}",
            path.display()
        );
    }
    for path in agent_asset_paths {
        assert!(path.starts_with(&user_root), "{}", path.display());
        assert!(!path.starts_with(&app_data), "{}", path.display());
        assert!(
            path.starts_with(paths.agent_assets_dir()),
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
    let legacy_agent_config = r#"name = "default"
description = "Legacy agent."
system_prompt = "Use the legacy prompt."
model = "legacy-model"
tools = []
skills = []
mcp_servers = []
"#;
    let legacy_prompt = "legacy compact prompt\n";

    std::fs::create_dir_all(paths.user_agents_dir()).expect("legacy dir");
    std::fs::create_dir_all(paths.legacy_agent_profiles_dir()).expect("legacy agents dir");
    std::fs::create_dir_all(paths.legacy_user_prompts_dir().join("compact"))
        .expect("legacy prompts dir");
    std::fs::write(paths.legacy_user_config_path(), legacy_user_config).expect("legacy config");
    std::fs::write(paths.legacy_providers_path(), legacy_provider_config).expect("legacy config");
    std::fs::write(
        paths.legacy_agent_profiles_dir().join("default.toml"),
        legacy_agent_config,
    )
    .expect("legacy agent config");
    std::fs::write(
        paths.legacy_user_prompts_dir().join("compact/default.md"),
        legacy_prompt,
    )
    .expect("legacy prompt");
    std::fs::write(paths.legacy_mcp_path(), "[servers.legacy]\n").expect("legacy mcp");
    std::fs::write(
        paths.legacy_shadowbox_path(),
        "workspace_boundary = false\n",
    )
    .expect("legacy shadowbox");

    init_user_data_at(&paths).await.expect("init user data");

    let migrated_user_config = std::fs::read_to_string(paths.user_config_path()).expect("config");
    assert_eq!(migrated_user_config, legacy_user_config);
    let migrated = std::fs::read_to_string(paths.providers_path()).expect("provider config");
    assert_eq!(migrated, legacy_provider_config);
    let migrated_agent =
        std::fs::read_to_string(paths.agent_profiles_dir().join("default.toml")).expect("agent");
    assert_eq!(migrated_agent, legacy_agent_config);
    let migrated_prompt =
        std::fs::read_to_string(paths.user_compact_prompts_dir().join("default.md"))
            .expect("prompt");
    assert_eq!(migrated_prompt, legacy_prompt);
    let migrated_mcp = std::fs::read_to_string(paths.mcp_path()).expect("mcp");
    assert_eq!(migrated_mcp, "[servers.legacy]\n");
    let migrated_shadowbox = std::fs::read_to_string(paths.shadowbox_path()).expect("shadowbox");
    assert_eq!(migrated_shadowbox, "workspace_boundary = false\n");
}
