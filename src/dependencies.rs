use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRecord {
    pub version: String,
    pub id: String,
    pub stage: String,
    pub owner: String,
    pub source_digest: String,
    pub dependencies: BTreeSet<String>,
}
#[derive(Debug, Clone, Copy)]
pub struct GraphLimits {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_depth: usize,
}
impl Default for GraphLimits {
    fn default() -> Self {
        Self {
            max_nodes: 1024,
            max_edges: 8192,
            max_depth: 64,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StageGraph {
    nodes: BTreeMap<String, StageRecord>,
    order: Vec<String>,
}
impl StageGraph {
    pub fn order(&self) -> &[String] {
        &self.order
    }
    pub fn nodes(&self) -> &BTreeMap<String, StageRecord> {
        &self.nodes
    }
}
pub fn build_graph(records: Vec<StageRecord>, limits: GraphLimits) -> Result<StageGraph, String> {
    if records.is_empty() || records.len() > limits.max_nodes || records.len() > 1024 {
        return Err("node budget".into());
    }
    let mut nodes = BTreeMap::new();
    let mut ownership = BTreeSet::new();
    let mut edge_count = 0usize;
    for record in records {
        if record.version != "flowguard.workflow/v1"
            || record.id.trim().is_empty()
            || record.owner.trim().is_empty()
            || !crate::valid_digest(&record.source_digest)
        {
            return Err("invalid stage identity/version/digest".into());
        }
        let project = crate::stage::STAGES
            .iter()
            .find(|(s, _)| *s == record.stage)
            .ok_or("unsupported stage")?
            .1;
        if project != (record.owner == "project") {
            return Err(format!("wrong owner: {}", record.id));
        }
        if !ownership.insert((record.owner.clone(), record.stage.clone())) {
            return Err(format!("duplicate ownership: {}", record.id));
        }
        edge_count = edge_count
            .checked_add(record.dependencies.len())
            .ok_or("edge budget")?;
        if edge_count > limits.max_edges || edge_count > 8192 {
            return Err("edge budget".into());
        }
        if nodes.insert(record.id.clone(), record).is_some() {
            return Err("duplicate node".into());
        }
    }
    for node in nodes.values() {
        for dep in &node.dependencies {
            if !nodes.contains_key(dep) {
                return Err(format!("dangling dependency {} -> {dep}", node.id));
            }
        }
    }
    let mut order = Vec::new();
    let mut depths = BTreeMap::<String, usize>::new();
    while order.len() < nodes.len() {
        let next = nodes.iter().find(|(id, n)| {
            !depths.contains_key(*id) && n.dependencies.iter().all(|d| depths.contains_key(d))
        });
        let Some((id, node)) = next else {
            return Err(format!(
                "cycle involving {}",
                nodes
                    .keys()
                    .filter(|id| !depths.contains_key(*id))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        };
        let depth = node
            .dependencies
            .iter()
            .map(|d| depths[d])
            .max()
            .unwrap_or(0)
            + 1;
        if depth > limits.max_depth || depth > 64 {
            return Err(format!("depth budget: {id}"));
        }
        depths.insert(id.clone(), depth);
        order.push(id.clone());
    }
    Ok(StageGraph { nodes, order })
}
