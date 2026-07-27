use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::{TaskPlan, SubTask, TaskStatus, TaskPriority};

/// Task Decomposer - phân tích task lớn thành subtasks
/// 
/// Enhanced with patterns from:
/// - **i-have-adhd**: ADHD-friendly task chunking, focus tracking, checkpoint/restore
/// - **agentmemory**: Task decomposition with priority scoring
/// - **code-review-graph**: Analysis-driven decomposition
pub struct TaskDecomposer {
    #[allow(dead_code)]
    max_subtasks: usize,
    focus_timer_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResult {
    pub task_plan: TaskPlan,
    pub analysis: TaskAnalysis,
    pub focus_plan: FocusPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusPlan {
    /// Estimated focus time per chunk (minutes)
    pub estimated_focus_minutes: u32,
    /// Number of focus blocks needed
    pub focus_blocks: u32,
    /// Break recommended between blocks (minutes)
    pub break_minutes: u32,
    /// ADHD-friendly action sequence
    pub action_sequence: Vec<String>,
    /// Checkpoints for saving progress
    pub checkpoints: Vec<String>,
    /// First action (must be doable in <2 min)
    pub first_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysis {
    pub task_type: TaskType,
    pub complexity: Complexity,
    pub estimated_steps: u32,
    pub requires_multi_agent: bool,
    pub suggested_agents: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    CodeGeneration,
    CodeReview,
    Debugging,
    Refactoring,
    Documentation,
    Architecture,
    Testing,
    DevOps,
    Research,
    General,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Complexity {
    Simple,       // 1-3 steps, <5min
    Medium,       // 4-8 steps, 5-15min
    Complex,      // 9-15 steps, 15-45min
    VeryComplex,  // 16+ steps, >45min
}

impl Complexity {
    pub fn estimated_minutes(&self) -> u32 {
        match self {
            Complexity::Simple => 5,
            Complexity::Medium => 15,
            Complexity::Complex => 30,
            Complexity::VeryComplex => 60,
        }
    }
}

impl TaskDecomposer {
    pub fn new() -> Self {
        Self { 
            max_subtasks: 10,
            focus_timer_minutes: 25, // Pomodoro-style
        }
    }

    /// Phân tích và decompose task với ADHD-friendly focus plan
    /// (from i-have-adhd: task chunking + focus management)
    pub fn decompose(&self, input: &str) -> Result<DecompositionResult> {
        let analysis = self.analyze_task(input);
        let task_plan = self.create_plan(input, &analysis);
        let focus_plan = self.create_focus_plan(&analysis);
        
        info!("Task decomposed: {:?} - {:?} ({})", 
            analysis.task_type, analysis.complexity, analysis.estimated_steps);
        
        Ok(DecompositionResult {
            task_plan,
            analysis,
            focus_plan,
        })
    }

    /// Create ADHD-friendly focus plan
    /// (inspired by i-have-adhd SKILL.md rules)
    fn create_focus_plan(&self, analysis: &TaskAnalysis) -> FocusPlan {
        let estimated_minutes = analysis.complexity.estimated_minutes();
        let focus_blocks = ((estimated_minutes as f32 / self.focus_timer_minutes as f32).ceil() as u32).max(1);
        
        // Build action sequence - numbered, bounded actions
        let action_sequence = match analysis.task_type {
            TaskType::CodeGeneration => vec![
                "Phân tích yêu cầu".into(),
                "Thiết kế giải pháp".into(),
                "Viết code chính".into(),
                "Review & test".into(),
            ],
            TaskType::Debugging => vec![
                "Tái hiện lỗi".into(),
                "Xác định nguyên nhân gốc".into(),
                "Sửa lỗi".into(),
                "Verify fix".into(),
            ],
            TaskType::CodeReview => vec![
                "Đọc code overview".into(),
                "Check logic & edge cases".into(),
                "Check security".into(),
                "Viết review comments".into(),
            ],
            _ => vec![
                "Phân tích".into(),
                "Thực hiện".into(),
                "Kiểm tra".into(),
            ],
        };

        // Checkpoints for saving/resuming progress
        let checkpoints = action_sequence.iter()
            .enumerate()
            .map(|(i, step)| format!("Checkpoint {}: {}", i + 1, step))
            .collect();

        // First action must be doable in <2 minutes (ADHD rule)
        let first_action = match analysis.task_type {
            TaskType::CodeGeneration => "Open your editor và tạo file mới".into(),
            TaskType::Debugging => "Chạy lại lệnh gây lỗi để capture stack trace".into(),
            TaskType::CodeReview => "Mở PR diff và đọc file đầu tiên".into(),
            TaskType::Refactoring => "Đọc code hiện tại ở module cần refactor".into(),
            TaskType::Documentation => "Mở file cần document".into(),
            TaskType::Architecture => "Vẽ sơ đồ kiến trúc hiện tại".into(),
            TaskType::Testing => "Mở file test đã có (nếu có)".into(),
            TaskType::DevOps => "Kiểm tra CI/CD status".into(),
            TaskType::Research => "Mở tab search với từ khóa chính".into(),
            _ => "Xác định bước đầu tiên cụ thể".into(),
        };

        FocusPlan {
            estimated_focus_minutes: estimated_minutes,
            focus_blocks,
            break_minutes: 5,
            action_sequence,
            checkpoints,
            first_action,
        }
    }

    /// Phân tích task type và complexity với keyword detection nâng cao
    fn analyze_task(&self, input: &str) -> TaskAnalysis {
        let lower = input.to_lowercase();
        
        let task_type = if lower.contains("create") || lower.contains("generate") || lower.contains("write") || lower.contains("build") || lower.contains("implement") || lower.contains("make") || lower.contains("new") {
            TaskType::CodeGeneration
        } else if lower.contains("review") || lower.contains("audit") || lower.contains("check") || lower.contains("inspect") {
            TaskType::CodeReview
        } else if lower.contains("fix") || lower.contains("bug") || lower.contains("error") || lower.contains("issue") || lower.contains("crash") || lower.contains("broken") {
            TaskType::Debugging
        } else if lower.contains("refactor") || lower.contains("optimize") || lower.contains("improve") || lower.contains("clean") || lower.contains("restructure") {
            TaskType::Refactoring
        } else if lower.contains("document") || lower.contains("doc") || lower.contains("readme") || lower.contains("explain") || lower.contains("comment") {
            TaskType::Documentation
        } else if lower.contains("design") || lower.contains("architecture") || lower.contains("plan") || lower.contains("blueprint") {
            TaskType::Architecture
        } else if lower.contains("test") || lower.contains("spec") {
            TaskType::Testing
        } else if lower.contains("deploy") || lower.contains("ci") || lower.contains("cd") || lower.contains("docker") || lower.contains("kubernetes") || lower.contains("infra") {
            TaskType::DevOps
        } else if lower.contains("research") || lower.contains("search") || lower.contains("find") || lower.contains("learn") || lower.contains("what is") || lower.contains("how to") {
            TaskType::Research
        } else {
            TaskType::General
        };

        let word_count = input.split_whitespace().count();
        let complexity = if word_count < 15 {
            Complexity::Simple
        } else if word_count < 40 {
            Complexity::Medium
        } else if word_count < 80 {
            Complexity::Complex
        } else {
            Complexity::VeryComplex
        };

        let requires_multi_agent = matches!(complexity, Complexity::Complex | Complexity::VeryComplex);
        
        let suggested_agents = self.suggest_agents(&task_type);
        let risks = self.identify_risks(&task_type, &complexity);

        TaskAnalysis {
            task_type,
            complexity,
            estimated_steps: match complexity {
                Complexity::Simple => 2,
                Complexity::Medium => 5,
                Complexity::Complex => 10,
                Complexity::VeryComplex => 15,
            },
            requires_multi_agent,
            suggested_agents,
            risks,
        }
    }

    /// Đề xuất agents phù hợp
    fn suggest_agents(&self, task_type: &TaskType) -> Vec<String> {
        match task_type {
            TaskType::CodeGeneration => vec![
                "senior_architect".into(),
                "backend_dev".into(),
                "frontend_dev".into(),
            ],
            TaskType::CodeReview => vec![
                "senior_architect".into(),
                "security_auditor".into(),
                "tester".into(),
            ],
            TaskType::Debugging => vec![
                "bug_fixer".into(),
                "tester".into(),
            ],
            TaskType::Refactoring => vec![
                "senior_architect".into(),
                "backend_dev".into(),
            ],
            TaskType::Documentation => vec![
                "senior_architect".into(),
            ],
            TaskType::Architecture => vec![
                "senior_architect".into(),
                "planner".into(),
                "security_auditor".into(),
            ],
            TaskType::Testing => vec![
                "tester".into(),
            ],
            TaskType::DevOps => vec![
                "backend_dev".into(),
                "security_auditor".into(),
            ],
            TaskType::Research => vec![
                "planner".into(),
            ],
            TaskType::General => vec![
                "planner".into(),
            ],
        }
    }

    /// Xác định risks
    fn identify_risks(&self, task_type: &TaskType, complexity: &Complexity) -> Vec<String> {
        let mut risks = Vec::new();
        
        match complexity {
            Complexity::Complex | Complexity::VeryComplex => {
                risks.push("Task complexity may require multiple iterations".into());
                risks.push("Consider breaking down into smaller steps".into());
            }
            _ => {}
        }

        match task_type {
            TaskType::CodeGeneration => {
                risks.push("Generated code needs review before production use".into());
            }
            TaskType::Debugging => {
                risks.push("Root cause may be deeper than apparent".into());
            }
            TaskType::Refactoring => {
                risks.push("Refactoring may introduce regressions".into());
            }
            _ => {}
        }

        risks
    }

    /// Tạo task plan từ analysis với ADHD-friendly subtask structure
    fn create_plan(&self, input: &str, analysis: &TaskAnalysis) -> TaskPlan {
        let task_id = uuid::Uuid::new_v4().to_string();
        
        let subtasks = match analysis.task_type {
            TaskType::CodeGeneration => vec![
                SubTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: "[1/4] Phân tích yêu cầu".into(),
                    description: "Đọc và phân tích chi tiết yêu cầu từ user, xác định scope và constraints".into(),
                    assigned_agent: Some("senior_architect".into()),
                    status: TaskStatus::Pending,
                    output: None,
                },
                SubTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: "[2/4] Thiết kế giải pháp".into(),
                    description: "Thiết kế architecture, tech stack, data flow. Xác định components".into(),
                    assigned_agent: Some("senior_architect".into()),
                    status: TaskStatus::Pending,
                    output: None,
                },
                SubTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: "[3/4] Implement code".into(),
                    description: "Viết code theo thiết kế. Từng module một".into(),
                    assigned_agent: Some("backend_dev".into()),
                    status: TaskStatus::Pending,
                    output: None,
                },
                SubTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: "[4/4] Review & test".into(),
                    description: "Chạy tests, review output, sửa lỗi nếu có".into(),
                    assigned_agent: Some("tester".into()),
                    status: TaskStatus::Pending,
                    output: None,
                },
            ],
            TaskType::Debugging => vec![
                SubTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: "[1/4] Reproduce lỗi".into(),
                    description: "Chạy lại đoạn code gây lỗi, capture đầy đủ error message và stack trace".into(),
                    assigned_agent: Some("bug_fixer".into()),
                    status: TaskStatus::Pending,
                    output: None,
                },
                SubTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: "[2/4] Tìm root cause".into(),
                    description: "Phân tích stack trace, check code path dẫn đến lỗi".into(),
                    assigned_agent: Some("bug_fixer".into()),
                    status: TaskStatus::Pending,
                    output: None,
                },
                SubTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: "[3/4] Implement fix".into(),
                    description: "Viết fix cho root cause, thêm error handling nếu cần".into(),
                    assigned_agent: Some("bug_fixer".into()),
                    status: TaskStatus::Pending,
                    output: None,
                },
                SubTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: "[4/4] Verify fix".into(),
                    description: "Chạy lại test case gốc, kiểm tra regression".into(),
                    assigned_agent: Some("tester".into()),
                    status: TaskStatus::Pending,
                    output: None,
                },
            ],
            _ => vec![
                SubTask {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: "Xử lý chính".into(),
                    description: input.to_string(),
                    assigned_agent: None,
                    status: TaskStatus::Pending,
                    output: None,
                },
            ],
        };

        TaskPlan {
            id: task_id,
            title: input.chars().take(50).collect(),
            description: input.to_string(),
            subtasks,
            dependencies: vec![],
            status: TaskStatus::Pending,
            priority: TaskPriority::Medium,
        }
    }
}
