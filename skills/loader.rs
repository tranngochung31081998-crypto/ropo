//! Skills loader — reads brain/knowledge files for each agent role.
//! Brain files live in `skills/{role}/skill.md` and are injected as
//! system prompt supplements before LLM calls.

use anyhow::Result;
use std::path::PathBuf;
use tracing::debug;

/// Agent role names — match directory names in skills/
pub const ROLE_ORCHESTRATOR: &str = "orchestrator";
pub const ROLE_CODER:        &str = "coder";
pub const ROLE_REVIEWER:     &str = "reviewer";
pub const ROLE_SECURITY:     &str = "security";
pub const ROLE_ARCHITECT:    &str = "architect";
pub const ROLE_DESIGNER:     &str = "designer";

pub struct SkillLoader {
    skills_dir: PathBuf,
    arch_dir:   PathBuf,
}

impl SkillLoader {
    pub fn new() -> Self {
        let skills_dir = std::env::var("CULI_SKILLS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("skills"));

        let arch_dir = PathBuf::from("docs/architecture");

        Self { skills_dir, arch_dir }
    }

    /// Load brain content for a role.
    /// Combines skill.md + optional context.md
    /// Returns empty string if role not found (graceful degradation).
    pub fn load_role(&self, role: &str) -> String {
        let mut out = String::new();

        let skill_file = self.skills_dir.join(role).join("skill.md");
        if skill_file.exists() {
            if let Ok(s) = std::fs::read_to_string(&skill_file) {
                debug!("SkillLoader: loaded {}", skill_file.display());
                out.push_str(&s);
                out.push_str("\n\n");
            }
        }

        let context_file = self.skills_dir.join(role).join("context.md");
        if context_file.exists() {
            if let Ok(s) = std::fs::read_to_string(&context_file) {
                out.push_str(&s);
            }
        }

        out
    }

    /// Load architecture overview (capped at 3000 chars to fit context budget).
    pub fn load_architecture_summary(&self) -> String {
        // Try human-readable markdown first
        let md = self.arch_dir.join("ARCHITECTURE.md");
        if md.exists() {
            if let Ok(s) = std::fs::read_to_string(&md) {
                // Cap at 3000 chars — context budget
                let cap = 3000;
                if s.len() > cap {
                    return format!(
                        "## Architecture Summary (truncated)\n{}...\n[See docs/architecture/ARCHITECTURE.md for full details]",
                        &s[..cap]
                    );
                }
                return s;
            }
        }
        String::new()
    }

    /// Build full system prompt for a role.
    /// = role brain + architecture context
    pub fn build_system_prompt(&self, role: &str, base_prompt: &str) -> String {
        let brain = self.load_role(role);
        let arch  = self.load_architecture_summary();

        let mut prompt = String::with_capacity(
            base_prompt.len() + brain.len() + arch.len() + 256
        );

        if !base_prompt.is_empty() {
            prompt.push_str(base_prompt);
            prompt.push_str("\n\n---\n\n");
        }

        if !brain.is_empty() {
            prompt.push_str("## Agent Brain\n\n");
            prompt.push_str(&brain);
            prompt.push_str("\n\n---\n\n");
        }

        if !arch.is_empty() {
            prompt.push_str("## Architecture Context\n\n");
            prompt.push_str(&arch);
            prompt.push_str("\n\n---\n\n");
        }

        prompt
    }

    /// Check if a role brain exists
    pub fn has_role(&self, role: &str) -> bool {
        self.skills_dir.join(role).join("skill.md").exists()
    }

    /// List all available roles
    pub fn available_roles(&self) -> Vec<String> {
        let mut roles = Vec::new();
        for r in &[
            ROLE_ORCHESTRATOR, ROLE_CODER, ROLE_REVIEWER,
            ROLE_SECURITY, ROLE_ARCHITECT, ROLE_DESIGNER,
        ] {
            if self.has_role(r) {
                roles.push(r.to_string());
            }
        }
        roles
    }
}

impl Default for SkillLoader {
    fn default() -> Self { Self::new() }
}
