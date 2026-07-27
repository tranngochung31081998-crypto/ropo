pub mod traits;
pub mod senior_architect;
pub mod planner;
pub mod security_auditor;
pub mod frontend_dev;
pub mod backend_dev;
pub mod tester;
pub mod bug_fixer;
pub mod ui_ux_designer;
pub mod harness;

pub use traits::*;
pub use senior_architect::*;
pub use planner::*;
pub use security_auditor::*;
pub use frontend_dev::*;
pub use backend_dev::*;
pub use tester::*;
pub use bug_fixer::*;
pub use ui_ux_designer::*;
pub use harness::*;

/// Agent type enum
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentType {
    SeniorArchitect,
    Planner,
    SecurityAuditor,
    FrontendDev,
    BackendDev,
    Tester,
    BugFixer,
    UiUxDesigner,
    Harness,
}

impl AgentType {
    pub fn to_string(&self) -> &str {
        match self {
            AgentType::SeniorArchitect => "senior_architect",
            AgentType::Planner => "planner",
            AgentType::SecurityAuditor => "security_auditor",
            AgentType::FrontendDev => "frontend_dev",
            AgentType::BackendDev => "backend_dev",
            AgentType::Tester => "tester",
            AgentType::BugFixer => "bug_fixer",
            AgentType::UiUxDesigner => "ui_ux_designer",
            AgentType::Harness => "harness",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            AgentType::SeniorArchitect => "Senior Software Architect - 15+ years experience, focuses on architecture, scalability, maintainability",
            AgentType::Planner => "Planning Analyst - Creates detailed plans, timelines, and task breakdowns",
            AgentType::SecurityAuditor => "Security Auditor - Finds vulnerabilities, performs attack & defense analysis",
            AgentType::FrontendDev => "Frontend Developer - Specializes in UI implementation, React, CSS, responsive design",
            AgentType::BackendDev => "Backend Developer - Specializes in APIs, databases, server logic, system design",
            AgentType::Tester => "QA Tester - Creates test cases, finds bugs, ensures quality",
            AgentType::BugFixer => "Bug Fixer - Debugs and fixes issues in code",
            AgentType::UiUxDesigner => "UI/UX Designer - Designs interfaces, design systems, color schemes, typography",
            AgentType::Harness => "Harness Agent - Handles context compression, chunk reading, graphify blast radius, and stateful memory updates",
        }
    }
}

/// Agent Pool - quản lý tất cả agents
pub struct AgentPool {
    pub senior_architect: SeniorArchitect,
    pub planner: Planner,
    pub security_auditor: SecurityAuditor,
    pub frontend_dev: FrontendDev,
    pub backend_dev: BackendDev,
    pub tester: Tester,
    pub bug_fixer: BugFixer,
    pub ui_ux_designer: UiUxDesigner,
    pub harness: HarnessAgent,
}

impl AgentPool {
    pub fn new() -> Self {
        Self {
            senior_architect: SeniorArchitect::new(),
            planner: Planner::new(),
            security_auditor: SecurityAuditor::new(),
            frontend_dev: FrontendDev::new(),
            backend_dev: BackendDev::new(),
            tester: Tester::new(),
            bug_fixer: BugFixer::new(),
            ui_ux_designer: UiUxDesigner::new(),
            harness: HarnessAgent::new(),
        }
    }

    pub fn get_agent(&self, agent_type: &AgentType) -> &dyn AgentRole {
        match agent_type {
            AgentType::SeniorArchitect => &self.senior_architect,
            AgentType::Planner => &self.planner,
            AgentType::SecurityAuditor => &self.security_auditor,
            AgentType::FrontendDev => &self.frontend_dev,
            AgentType::BackendDev => &self.backend_dev,
            AgentType::Tester => &self.tester,
            AgentType::BugFixer => &self.bug_fixer,
            AgentType::UiUxDesigner => &self.ui_ux_designer,
            AgentType::Harness => &self.harness,
        }
    }
}
