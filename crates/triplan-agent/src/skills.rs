use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;
use walkdir::WalkDir;

use crate::config::{runtime_paths, RuntimePaths};
use crate::error::{AgentError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LoadedSkill {
    pub metadata: SkillMetadata,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct SkillRegistry {
    skills: Vec<SkillMetadata>,
}

impl SkillRegistry {
    pub async fn scan(_workspace: &Path) -> Result<Self> {
        let paths = runtime_paths()?;
        Self::scan_user_data(&paths).await
    }

    pub async fn scan_user_data(paths: &RuntimePaths) -> Result<Self> {
        Self::scan_agent_dir(paths.user_config_dir()).await
    }

    pub async fn scan_agent_dir(agent_dir: &Path) -> Result<Self> {
        let root = agent_dir.join("skills");
        let mut skills = Vec::new();
        if !root.exists() {
            return Ok(Self { skills });
        }
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if entry.file_name() != "SKILL.md" {
                continue;
            }
            let path = entry.path().to_path_buf();
            let content = fs::read_to_string(&path).await?;
            let metadata = parse_skill_metadata(&content, path)?;
            skills.push(metadata);
        }
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self { skills })
    }

    pub fn metadata(&self) -> &[SkillMetadata] {
        &self.skills
    }

    pub async fn load(&self, name: &str) -> Result<LoadedSkill> {
        let metadata = self
            .skills
            .iter()
            .find(|skill| skill.name == name)
            .cloned()
            .ok_or_else(|| AgentError::NotFound(format!("skill {name}")))?;
        let content = fs::read_to_string(&metadata.path).await?;
        let body = content
            .split_once("---")
            .and_then(|(_, rest)| rest.split_once("---"))
            .map(|(_, body)| body.to_string())
            .unwrap_or(content);
        Ok(LoadedSkill { metadata, body })
    }
}

fn parse_skill_metadata(content: &str, path: PathBuf) -> Result<SkillMetadata> {
    let (_, rest) = content
        .split_once("---")
        .ok_or_else(|| AgentError::Config("skill missing front matter".to_string()))?;
    let (front_matter, _) = rest
        .split_once("---")
        .ok_or_else(|| AgentError::Config("skill missing closing front matter".to_string()))?;
    let name = front_matter
        .lines()
        .find_map(|line| line.strip_prefix("name:").map(str::trim))
        .ok_or_else(|| AgentError::Config("skill missing name".to_string()))?;
    let description = front_matter
        .lines()
        .find_map(|line| line.strip_prefix("description:").map(str::trim))
        .ok_or_else(|| AgentError::Config("skill missing description".to_string()))?;
    Ok(SkillMetadata {
        name: name.to_string(),
        description: description.to_string(),
        path,
    })
}
