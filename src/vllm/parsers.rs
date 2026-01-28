use crate::parsers::{build_file_path, Metadata, ParserOutput, ParserResults, StructuredLogParser};
use crate::templates::TEMPLATE_QUERY_PARAM_SCRIPT;
use crate::types::{CompileId, Envelope};

use super::types::{
    ArtifactInfo, VllmCompilationConfig, VllmCompileRangeGroup, VllmSubgraphInfo,
    VllmSubgraphWithArtifacts, VllmSummaryContext,
};

use std::cell::RefCell;
use std::rc::Rc;
use tinytemplate::TinyTemplate;

#[derive(Debug, Default)]
pub struct VllmState {
    pub config: RefCell<Option<VllmCompilationConfig>>,
    pub piecewise_graph_file: RefCell<Option<String>>,
    pub subgraphs: RefCell<Vec<VllmSubgraphInfo>>,
    pub pre_subgraph_artifacts: RefCell<Vec<ArtifactInfo>>,
    pub has_vllm_artifacts: RefCell<bool>,
}

impl VllmState {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn has_artifacts(&self) -> bool {
        *self.has_vllm_artifacts.borrow()
    }

    // Add artifact to current subgraph, or pre_subgraph_artifacts if no subgraph yet
    pub fn add_artifact(&self, filename: &std::path::Path, suffix: String) {
        let url = filename.to_string_lossy().to_string();
        let name = filename
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| url.clone());

        // Track piecewise split graph file for linking in summary
        if name.starts_with("vllm_piecewise_split_graph") {
            *self.piecewise_graph_file.borrow_mut() = Some(url.clone());
        }

        let artifact = ArtifactInfo { name, url, suffix };
        let mut subgraphs = self.subgraphs.borrow_mut();
        if let Some(last) = subgraphs.last_mut() {
            last.artifacts.push(artifact);
        } else {
            self.pre_subgraph_artifacts.borrow_mut().push(artifact);
        }
    }

    // Group subgraphs by compile range/size for hierarchical display
    pub fn build_compile_range_groups(&self) -> Vec<VllmCompileRangeGroup> {
        use indexmap::IndexMap;

        let subgraphs = self.subgraphs.borrow();
        let mut groups: IndexMap<String, Vec<VllmSubgraphWithArtifacts>> = IndexMap::new();

        for subgraph in subgraphs.iter() {
            let size_or_range = subgraph.size_or_range();
            let artifact_count = subgraph.artifacts.len();
            groups
                .entry(size_or_range)
                .or_default()
                .push(VllmSubgraphWithArtifacts {
                    submod_name: subgraph.display_submod_name(),
                    artifacts: subgraph.artifacts.clone(),
                    artifact_count,
                });
        }

        groups
            .into_iter()
            .map(|(size_or_range, submods)| VllmCompileRangeGroup {
                size_or_range,
                submod_count: submods.len(),
                submods,
            })
            .collect()
    }

    // Get dynamo artifacts from pre_subgraph_artifacts
    pub fn build_dynamo_artifacts(&self) -> Vec<ArtifactInfo> {
        let dynamo_names = [
            "dynamo_side_effects",
            "dynamo_output_graph",
            "dynamo_cpp_guards_str",
            "compilation_metrics",
        ];
        self.pre_subgraph_artifacts
            .borrow()
            .iter()
            .filter(|a| dynamo_names.iter().any(|name| a.name.starts_with(name)))
            .cloned()
            .collect()
    }
}

// Parses vllm_compilation_config artifacts.
// Stores config in state for display, outputs formatted JSON file.
pub struct VllmCompilationConfigParser {
    state: Rc<VllmState>,
}

impl VllmCompilationConfigParser {
    pub fn new(state: Rc<VllmState>) -> Self {
        Self { state }
    }
}

impl StructuredLogParser for VllmCompilationConfigParser {
    fn name(&self) -> &'static str {
        "vllm_compilation_config"
    }

    fn get_metadata<'e>(&self, e: &'e Envelope) -> Option<Metadata<'e>> {
        if let Some(artifact) = &e.artifact {
            if artifact.name == "vllm_compilation_config" {
                return Some(Metadata::Artifact(artifact));
            }
        }
        None
    }

    fn parse<'e>(
        &self,
        lineno: usize,
        _metadata: Metadata<'e>,
        _rank: Option<u32>,
        compile_id: &Option<CompileId>,
        payload: &str,
    ) -> anyhow::Result<ParserResults> {
        if let Ok(config) = serde_json::from_str::<VllmCompilationConfig>(payload) {
            *self.state.config.borrow_mut() = Some(config);
            *self.state.has_vllm_artifacts.borrow_mut() = true;
        }

        let f = build_file_path("vllm_compilation_config.json", lineno, compile_id);
        Ok(vec![ParserOutput::PayloadReformatFile(f, |payload| {
            let value: serde_json::Value = serde_json::from_str(payload)?;
            Ok(serde_json::to_string_pretty(&value)?)
        })])
    }
}

// Parses vllm_piecewise_compile_start artifacts and vllm_subgraph_*/vllm_submod_* graph dumps.
// On compile_start: pushes new VllmSubgraphInfo to state.subgraphs (subsequent artifacts attach here).
// On graph_dump: adds artifact to current subgraph and outputs the graph file.
pub struct VllmPiecewiseCompileParser {
    state: Rc<VllmState>,
}

impl VllmPiecewiseCompileParser {
    pub fn new(state: Rc<VllmState>) -> Self {
        Self { state }
    }
}

impl StructuredLogParser for VllmPiecewiseCompileParser {
    fn name(&self) -> &'static str {
        "vllm_piecewise_compile"
    }

    fn get_metadata<'e>(&self, e: &'e Envelope) -> Option<Metadata<'e>> {
        if let Some(artifact) = &e.artifact {
            if artifact.name == "vllm_piecewise_compile_start" {
                return Some(Metadata::Artifact(artifact));
            }
        }
        if let Some(graph_dump) = &e.graph_dump {
            if graph_dump.name.starts_with("vllm_subgraph_")
                || graph_dump.name.starts_with("vllm_submod_")
            {
                return Some(Metadata::GraphDump(graph_dump));
            }
        }
        None
    }

    fn parse<'e>(
        &self,
        lineno: usize,
        metadata: Metadata<'e>,
        _rank: Option<u32>,
        compile_id: &Option<CompileId>,
        payload: &str,
    ) -> anyhow::Result<ParserResults> {
        *self.state.has_vllm_artifacts.borrow_mut() = true;

        match metadata {
            Metadata::Artifact(_artifact) => {
                if let Ok(subgraph) = serde_json::from_str::<VllmSubgraphInfo>(payload) {
                    self.state.subgraphs.borrow_mut().push(subgraph);
                }
                Ok(Vec::new())
            }
            Metadata::GraphDump(graph_dump) => {
                let name = &graph_dump.name;
                let filename = format!("{}.txt", name);
                let f = build_file_path(&filename, lineno, compile_id);
                // add_file_output will call add_artifact for us
                Ok(vec![ParserOutput::PayloadFile(f)])
            }
            _ => Ok(Vec::new()),
        }
    }
}

// Parses vllm_piecewise_split_graph graph dumps.
// Stores path in state for linking in summary, outputs the graph file.
pub struct VllmPiecewiseSplitGraphParser {
    state: Rc<VllmState>,
}

impl VllmPiecewiseSplitGraphParser {
    pub fn new(state: Rc<VllmState>) -> Self {
        Self { state }
    }
}

impl StructuredLogParser for VllmPiecewiseSplitGraphParser {
    fn name(&self) -> &'static str {
        "vllm_piecewise_split_graph"
    }

    fn get_metadata<'e>(&self, e: &'e Envelope) -> Option<Metadata<'e>> {
        if let Some(graph_dump) = &e.graph_dump {
            if graph_dump.name == "vllm_piecewise_split_graph" {
                return Some(Metadata::GraphDump(graph_dump));
            }
        }
        None
    }

    fn parse<'e>(
        &self,
        lineno: usize,
        _metadata: Metadata<'e>,
        _rank: Option<u32>,
        compile_id: &Option<CompileId>,
        _payload: &str,
    ) -> anyhow::Result<ParserResults> {
        let filename = "vllm_piecewise_split_graph.txt";
        let f = build_file_path(filename, lineno, compile_id);
        *self.state.has_vllm_artifacts.borrow_mut() = true;
        Ok(vec![ParserOutput::PayloadFile(f)])
    }
}

pub fn vllm_parsers_with_state(state: Rc<VllmState>) -> Vec<Box<dyn StructuredLogParser>> {
    vec![
        Box::new(VllmCompilationConfigParser::new(state.clone())),
        Box::new(VllmPiecewiseSplitGraphParser::new(state.clone())),
        Box::new(VllmPiecewiseCompileParser::new(state.clone())),
    ]
}

pub fn generate_vllm_summary(
    state: &VllmState,
    tt: &TinyTemplate,
    custom_header_html: &str,
) -> anyhow::Result<String> {
    let config = state.config.borrow().clone().unwrap_or_default();
    let dynamo_artifacts = state.build_dynamo_artifacts();
    let has_dynamo_artifacts = !dynamo_artifacts.is_empty();
    let piecewise_graph_file = state.piecewise_graph_file.borrow().clone();
    let has_piecewise = piecewise_graph_file.is_some();
    let compile_range_groups = state.build_compile_range_groups();

    let context = VllmSummaryContext {
        css: super::templates::VLLM_CSS.to_string(),
        qps: TEMPLATE_QUERY_PARAM_SCRIPT.to_string(),
        custom_header_html: custom_header_html.to_string(),
        has_config: state.config.borrow().is_some(),
        config,
        dynamo_artifacts,
        has_dynamo_artifacts,
        piecewise_graph_file,
        has_piecewise,
        compile_range_groups,
    };

    Ok(tt.render("vllm_summary.html", &context)?)
}
