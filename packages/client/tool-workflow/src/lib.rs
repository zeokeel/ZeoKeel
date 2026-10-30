//! Linear/DAG workflow over subagents.
#![forbid(unsafe_code)]
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct WorkflowTool;
#[must_use]
pub fn tool() -> WorkflowTool {
    WorkflowTool
}

struct Step {
    id: String,
    prompt: String,
    depends_on: Vec<String>,
}

fn parse_steps(value: &serde_json::Value) -> Result<Vec<Step>, ToolError> {
    let items = value
        .as_array()
        .ok_or_else(|| ToolError::InvalidArgs("steps".to_owned()))?;
    if items.is_empty() {
        return Err(ToolError::InvalidArgs("workflow has no steps".to_owned()));
    }
    if items.len() > 20 {
        return Err(ToolError::InvalidArgs(
            "workflow too long (max 20)".to_owned(),
        ));
    }
    let mut steps = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let prompt = item
            .get("prompt")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("step prompt".to_owned()))?;
        let id = item
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("step-{}", index.saturating_add(1)));
        let depends_on = item
            .get("depends_on")
            .and_then(serde_json::Value::as_array)
            .map(|deps| {
                deps.iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        steps.push(Step {
            id,
            prompt: prompt.to_owned(),
            depends_on,
        });
    }
    let mut ids = BTreeSet::new();
    for step in &steps {
        if !ids.insert(step.id.clone()) {
            return Err(ToolError::InvalidArgs(format!(
                "duplicate step id {}",
                step.id
            )));
        }
    }
    for step in &steps {
        for dep in &step.depends_on {
            if !ids.contains(dep) {
                return Err(ToolError::InvalidArgs(format!("unknown dependency {dep}")));
            }
        }
    }
    Ok(steps)
}

fn order(steps: &[Step]) -> Result<Vec<usize>, ToolError> {
    let mut indegree: BTreeMap<&str, usize> = steps
        .iter()
        .map(|step| (step.id.as_str(), step.depends_on.len()))
        .collect();
    let mut outgoing: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, step) in steps.iter().enumerate() {
        for dep in &step.depends_on {
            outgoing.entry(dep.as_str()).or_default().push(index);
        }
    }
    let mut queue: VecDeque<usize> = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| step.depends_on.is_empty())
        .map(|(index, _)| index)
        .collect();
    let mut result = Vec::with_capacity(steps.len());
    while let Some(index) = queue.pop_front() {
        result.push(index);
        let Some(step) = steps.get(index) else {
            continue;
        };
        if let Some(children) = outgoing.get(step.id.as_str()) {
            for &child in children {
                let Some(child_step) = steps.get(child) else {
                    continue;
                };
                if let Some(value) = indegree.get_mut(child_step.id.as_str()) {
                    *value = value.saturating_sub(1);
                    if *value == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }
    }
    if result.len() != steps.len() {
        return Err(ToolError::InvalidArgs(
            "workflow has a dependency cycle".to_owned(),
        ));
    }
    Ok(result)
}

impl Tool for WorkflowTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec { name: "workflow".to_owned(), description: "Run a workflow: each step prompt is delegated to a child agent in dependency order (DAG).".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"steps":{"type":"array"}},"required":["steps"],"additionalProperties":false}), mutating: false, requires_approval: false }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs("workflow requires JSON".to_owned()))
    }
    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let runner = zk_kit_subagent::runner()
            .ok_or_else(|| ToolError::Denied("subagent runner is not installed".to_owned()))?;
        let steps = parse_steps(args.get("steps").unwrap_or(&serde_json::Value::Null))?;
        let order = order(&steps)?;
        let depth = zk_kit_subagent::current_depth();
        let mut results: BTreeMap<String, String> = BTreeMap::new();
        let mut out = Vec::new();
        for index in order {
            let step = steps
                .get(index)
                .ok_or_else(|| ToolError::Io("workflow index invalid".to_owned()))?;
            let deps: Vec<String> = step
                .depends_on
                .iter()
                .filter_map(|id| results.get(id).map(|text| format!("{id}:\n{text}")))
                .collect();
            let prompt = if deps.is_empty() {
                step.prompt.clone()
            } else {
                format!(
                    "{}\n\nDependency results:\n{}",
                    step.prompt,
                    deps.join("\n\n")
                )
            };
            let text = zk_kit_subagent::with_depth(depth.saturating_add(1), || {
                runner.run(&prompt, context, depth)
            })
            .map_err(ToolError::Io)?;
            results.insert(step.id.clone(), text.clone());
            out.push(serde_json::json!({"id":step.id,"depends_on":step.depends_on,"result":text}));
        }
        Ok(serde_json::json!({"steps":out}).to_string())
    }
}
