use std::collections::HashMap;
use anyhow::Result;
use tracing::info;

use super::Skill;

/// Skill Registry - quản lý và discover skills
/// Inspired by Hermes Agent's skill system và agent-skills
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    #[allow(dead_code)]
    categories: HashMap<String, Vec<String>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            categories: HashMap::new(),
        }
    }

    /// Register a skill
    pub fn register(&mut self, skill: Skill) -> Result<()> {
        let id = skill.id.clone();
        let name = skill.name.clone();
        info!("Registering skill: {} ({})", name, id);
        self.skills.insert(id.clone(), skill);
        Ok(())
    }

    /// Get skill by ID
    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.get(id)
    }

    /// Find skill by name
    pub fn find_by_name(&self, name: &str) -> Option<&Skill> {
        self.skills.values().find(|s| s.name == name)
    }

    /// Search skills by keyword
    pub fn search(&self, query: &str) -> Vec<&Skill> {
        let query_lower = query.to_lowercase();
        self.skills.values()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower) ||
                s.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Get all skills
    pub fn all(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    /// Get skills by type
    pub fn by_type(&self, skill_type: &super::SkillType) -> Vec<&Skill> {
        self.skills.values()
            .filter(|s| s.skill_type == *skill_type)
            .collect()
    }
}
