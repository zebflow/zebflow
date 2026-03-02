//! Prompt templates for strategic planning.

/// Prompt for decomposing objective into subgoals.
pub fn decompose_objective_prompt(objective: &str) -> String {
    format!(
        r#"You are a strategic planning assistant. Break down this objective into 3-5 concrete subgoals.

Objective: {objective}

For each subgoal provide:
1. Description (what needs to be done)
2. Success criteria (how to validate it's complete)
3. Initial steps (1-3 actions using available tools)

Format your response as:
SUBGOAL 1: [description]
SUCCESS: [validation criteria]
STEPS:
- [step 1]
- [step 2]

SUBGOAL 2: [description]
...

Keep subgoals:
- Concrete and actionable
- Sequential (each builds on previous)
- Measurable (clear success criteria)
- Tool-focused (use specific tool names)

Available tools: ls, pwd, python, read_file, write_file, find_files, grep, shell, git_status, git_log, git_diff, web_search, web_fetch"#
    )
}

/// Prompt for validating subgoal completion.
pub fn validate_subgoal_prompt(subgoal: &str, criteria: &str, tool_results: &str) -> String {
    format!(
        r#"Did we successfully complete this subgoal?

Subgoal: {subgoal}
Success criteria: {criteria}

Tool results:
{tool_results}

Answer with:
SUCCESS: YES or NO
CONFIDENCE: 0.0 to 1.0
REASON: [explanation]

Be strict - only say YES if the criteria is clearly met."#
    )
}

/// Prompt for replanning after failure.
pub fn replan_subgoal_prompt(
    subgoal: &str,
    failed_steps: &str,
    error: &str,
    attempts: u32,
) -> String {
    format!(
        r#"The previous attempt failed. Create a NEW plan to achieve this subgoal.

Subgoal: {subgoal}
Previous attempt #{attempts}

Failed steps:
{failed_steps}

Error/issue:
{error}

Provide alternative steps (1-3 actions) using different approach:
STEPS:
- [step 1]
- [step 2]

Focus on:
- Using different tools if previous failed
- Breaking down into smaller actions
- Avoiding the same error"#
    )
}

/// Prompt for final goal validation.
pub fn validate_final_goal_prompt(objective: &str, all_results: &str) -> String {
    format!(
        r#"Did we achieve the original objective?

Objective: {objective}

All results from subgoals:
{all_results}

Answer with:
SUCCESS: YES or NO
CONFIDENCE: 0.0 to 1.0
SUMMARY: [brief summary of what was accomplished]

Be honest - if objective is only partially met, say NO."#
    )
}
