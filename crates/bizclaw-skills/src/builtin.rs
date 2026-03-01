//! Built-in skills — bundled with BizClaw.

use crate::parser::SkillManifest;

/// Get all built-in skills.
pub fn builtin_skills() -> Vec<SkillManifest> {
    let skill_defs = vec![
        (
            "rust-expert",
            "Rust Expert",
            "Deep expertise in Rust programming, ownership, async, traits",
            "coding",
            "🦀",
            vec!["rust", "programming", "systems"],
            include_str!("skills/rust_expert.md"),
        ),
        (
            "python-analyst",
            "Python Analyst",
            "Python data analysis, pandas, numpy, visualization",
            "data",
            "🐍",
            vec!["python", "data", "analytics"],
            include_str!("skills/python_analyst.md"),
        ),
        (
            "web-developer",
            "Web Developer",
            "Full-stack web development with modern frameworks",
            "coding",
            "🌐",
            vec!["web", "javascript", "html", "css", "react"],
            include_str!("skills/web_developer.md"),
        ),
        (
            "devops-engineer",
            "DevOps Engineer",
            "CI/CD, Docker, Kubernetes, infrastructure automation",
            "devops",
            "🔧",
            vec!["devops", "docker", "kubernetes", "ci-cd"],
            include_str!("skills/devops_engineer.md"),
        ),
        (
            "content-writer",
            "Content Writer",
            "Professional writing for blogs, marketing, social media",
            "writing",
            "✍️",
            vec!["writing", "content", "marketing", "seo"],
            include_str!("skills/content_writer.md"),
        ),
        (
            "security-auditor",
            "Security Auditor",
            "Code security review, vulnerability assessment, best practices",
            "security",
            "🔒",
            vec!["security", "audit", "vulnerability", "owasp"],
            include_str!("skills/security_auditor.md"),
        ),
        (
            "sql-expert",
            "SQL Expert",
            "Database design, query optimization, PostgreSQL, SQLite",
            "data",
            "🗄️",
            vec!["sql", "database", "postgresql", "sqlite"],
            include_str!("skills/sql_expert.md"),
        ),
        (
            "api-designer",
            "API Designer",
            "RESTful API design, OpenAPI, GraphQL, gRPC",
            "coding",
            "🔌",
            vec!["api", "rest", "graphql", "grpc"],
            include_str!("skills/api_designer.md"),
        ),
        (
            "vietnamese-business",
            "Vietnamese Business",
            "Luật doanh nghiệp Việt Nam, thuế, lao động, hóa đơn",
            "business",
            "🇻🇳",
            vec!["vietnam", "business", "legal", "tax"],
            include_str!("skills/vietnamese_business.md"),
        ),
        (
            "git-workflow",
            "Git Workflow",
            "Git branching strategies, code review, CI/CD integration",
            "devops",
            "📦",
            vec!["git", "version-control", "github", "workflow"],
            include_str!("skills/git_workflow.md"),
        ),
    ];

    skill_defs
        .into_iter()
        .filter_map(|(name, display, desc, cat, icon, tags, content)| {
            let yaml = format!(
                "---\nname: {}\ndisplay_name: {}\ndescription: {}\ncategory: {}\nicon: {}\ntags: [{}]\n---\n{}",
                name,
                display,
                desc,
                cat,
                icon,
                tags.join(", "),
                content,
            );
            SkillManifest::parse(&yaml).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_skills_count() {
        let skills = builtin_skills();
        assert_eq!(skills.len(), 10);
    }

    #[test]
    fn test_builtin_skills_have_content() {
        for skill in builtin_skills() {
            assert!(
                !skill.content.is_empty(),
                "Skill '{}' has no content",
                skill.metadata.name
            );
            assert!(
                !skill.metadata.tags.is_empty(),
                "Skill '{}' has no tags",
                skill.metadata.name
            );
        }
    }
}
