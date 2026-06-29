//! Built-in skill bundles shipped inside the app. A bundle is a curated set of
//! `SKILL.md` files that can be installed into the active project/workspace skill
//! directories (`.claude/skills`, `.agents/skills`, …) with one click — no network,
//! no `npx skills`. The actual install (writing the files) lives in `solution.rs`,
//! which already owns the workspace-skill helpers; this module is just the data.

use serde::{Deserialize, Serialize};

/// One skill inside a bundle: the directory name plus its embedded `SKILL.md`.
pub struct BundleSkill {
    pub name: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

/// A named, curated set of skills compiled into the binary.
pub struct SkillBundle {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub skills: &'static [BundleSkill],
}

const DEV_WORKFLOW: SkillBundle = SkillBundle {
    id: "dev-workflow",
    label: "Dev Workflow",
    description: "A plan → implement → review → commit loop for everyday code changes.",
    skills: &[
        BundleSkill {
            name: "plan",
            description: "Plan a code change before writing it.",
            content: include_str!("skill_bundles/dev_workflow/plan.md"),
        },
        BundleSkill {
            name: "implement",
            description: "Implement an approved plan with focused edits.",
            content: include_str!("skill_bundles/dev_workflow/implement.md"),
        },
        BundleSkill {
            name: "review",
            description: "Self-review a diff before committing.",
            content: include_str!("skill_bundles/dev_workflow/review.md"),
        },
        BundleSkill {
            name: "commit",
            description: "Write a clean commit and a clear PR.",
            content: include_str!("skill_bundles/dev_workflow/commit.md"),
        },
    ],
};

/// Every bundle the app ships. Add new bundles here.
pub const BUNDLES: &[&SkillBundle] = &[&DEV_WORKFLOW];

pub fn find_bundle(id: &str) -> Option<&'static SkillBundle> {
    BUNDLES.iter().copied().find(|bundle| bundle.id == id)
}

/// Serializable summary for the frontend (no file contents).
#[derive(Debug, Clone, Serialize)]
pub struct SkillBundleInfo {
    pub id: String,
    pub label: String,
    pub description: String,
    pub skills: Vec<SkillBundleSkillInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillBundleSkillInfo {
    pub name: String,
    pub description: String,
}

pub fn list_bundles() -> Vec<SkillBundleInfo> {
    BUNDLES
        .iter()
        .map(|bundle| SkillBundleInfo {
            id: bundle.id.to_string(),
            label: bundle.label.to_string(),
            description: bundle.description.to_string(),
            skills: bundle
                .skills
                .iter()
                .map(|skill| SkillBundleSkillInfo {
                    name: skill.name.to_string(),
                    description: skill.description.to_string(),
                })
                .collect(),
        })
        .collect()
}

/// Where to install a bundle and which agent targets to write.
#[derive(Debug, Clone, Deserialize)]
pub struct InstallSkillBundleInput {
    /// Project or workspace directory to install into.
    pub workspace_path: String,
    pub bundle_id: String,
    /// Agent targets (e.g. ["claude", "codex"]). Empty → all defaults.
    #[serde(default)]
    pub targets: Vec<String>,
}
