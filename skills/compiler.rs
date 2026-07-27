use anyhow::Result;
use tracing::info;

use super::{Skill, SkillStep};

/// Skill Compiler - compile skill definitions từ markdown/code
/// Inspired by agent-skills SKILL.md format
pub struct SkillCompiler;

impl SkillCompiler {
    pub fn new() -> Self {
        Self
    }

    /// Parse SKILL.md frontmatter + steps
    pub fn parse_markdown(&self, source: &str) -> Result<Skill> {
        info!("Compiling skill from markdown source...");
        let lines: Vec<&str> = source.lines().collect();

        let mut skill = Skill::new("untitled", "");
        let mut in_frontmatter = false;
        let mut in_steps = false;
        let mut step_order = 0u32;

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.starts_with("---") && !in_frontmatter {
                in_frontmatter = true;
                continue;
            }
            if trimmed.starts_with("---") && in_frontmatter {
                in_frontmatter = false;
                continue;
            }

            if in_frontmatter {
                if let Some((key, value)) = trimmed.split_once(':').map(|(k, v)| (k.trim(), v.trim())) {
                    match key {
                        "name" => skill.name = value.to_string(),
                        "description" => skill.description = value.to_string(),
                        "version" => skill.version = value.to_string(),
                        "author" => skill.author = value.to_string(),
                        _ => {}
                    }
                }
                continue;
            }

            if trimmed.starts_with("## Steps") || trimmed.starts_with("## Process") {
                in_steps = true;
                continue;
            }

            if in_steps && trimmed.starts_with("### ") {
                step_order += 1;
                let step_name = trimmed.trim_start_matches("### ").to_string();
                skill.steps.push(SkillStep {
                    order: step_order,
                    name: step_name,
                    action: "execute".to_string(),
                    params: std::collections::HashMap::new(),
                    condition: None,
                    timeout_secs: None,
                });
            }
        }

        info!("Compiled skill: {} ({} steps)", skill.name, skill.steps.len());
        Ok(skill)
    }

    /// Serialize skill to SKILL.md format
    pub fn to_markdown(&self, skill: &Skill) -> String {
        let mut output = String::new();
        output.push_str("---\n");
        output.push_str(&format!("name: {}\n", skill.name));
        output.push_str(&format!("description: {}\n", skill.description));
        output.push_str(&format!("version: {}\n", skill.version));
        output.push_str(&format!("author: {}\n", skill.author));
        output.push_str("---\n\n");
        output.push_str(&format!("# {}\n\n", skill.name));
        output.push_str(&format!("{}\n\n", skill.description));
        output.push_str("## Steps\n\n");
        for step in &skill.steps {
            output.push_str(&format!("### {}\n\n", step.name));
            output.push_str(&format!("Action: {}\n\n", step.action));
        }
        output
    }
}
