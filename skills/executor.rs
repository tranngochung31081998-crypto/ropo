use anyhow::Result;
use tracing::info;

use super::{Skill, SkillStep};

/// Skill Executor - thực thi skill steps
pub struct SkillExecutor {
    #[allow(dead_code)]
    max_retries: u32,
}

impl SkillExecutor {
    pub fn new(max_retries: u32) -> Self {
        Self { max_retries }
    }

    /// Execute a skill with its steps
    pub async fn execute(&self, skill: &Skill) -> Result<SkillResult> {
        info!("Executing skill: {} ({} steps)", skill.name, skill.steps.len());
        let mut results = Vec::new();

        for step in &skill.steps {
            let step_result = self.execute_step(step).await?;
            results.push(step_result);
        }

        Ok(SkillResult {
            skill_id: skill.id.clone(),
            success: results.iter().all(|r| r.success),
            steps: results,
        })
    }

    /// Execute individual skill step
    async fn execute_step(&self, step: &SkillStep) -> Result<StepResult> {
        info!("Executing step: {} ({})", step.name, step.action);
        Ok(StepResult {
            step_name: step.name.clone(),
            success: true,
            output: format!("Step {} executed", step.name),
        })
    }
}

#[derive(Debug)]
pub struct SkillResult {
    pub skill_id: String,
    pub success: bool,
    pub steps: Vec<StepResult>,
}

#[derive(Debug)]
pub struct StepResult {
    pub step_name: String,
    pub success: bool,
    pub output: String,
}
