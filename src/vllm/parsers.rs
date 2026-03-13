use crate::parsers::{build_file_path, Metadata, ParserOutput, ParserResults, StructuredLogParser};
use crate::templates::TEMPLATE_QUERY_PARAM_SCRIPT;
use crate::types::{CompileId, Envelope};

use super::types::{
    ArtifactInfo, VllmCompilationConfig, VllmCompileCall, VllmCompileCallContext,
    VllmCompileRangeGroup, VllmDiffContext, VllmSubgraphInfo, VllmSubgraphWithArtifacts,
    VllmSummaryContext,
};

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::rc::Rc;
use tinytemplate::TinyTemplate;

#[derive(Debug, Default)]
pub struct VllmState {
    pub compile_calls: RefCell<Vec<VllmCompileCall>>,
    pub has_vllm_artifacts: RefCell<bool>,
}

impl VllmState {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn has_artifacts(&self) -> bool {
        *self.has_vllm_artifacts.borrow()
    }

    /// Ensure at least one compile call exists, creating a default one if needed.
    fn ensure_compile_call(&self) {
        let mut calls = self.compile_calls.borrow_mut();
        if calls.is_empty() {
            calls.push(VllmCompileCall {
                index: 0,
                ..Default::default()
            });
        }
    }

    /// Start a new compile call with the given config.
    /// Start a new compile call with the given config.
    /// If the current (last) compile call has no config yet (created by ensure_compile_call
    /// for early artifacts), populate it instead of creating a new one.
    pub fn push_compile_call(&self, config: VllmCompilationConfig) {
        let mut calls = self.compile_calls.borrow_mut();
        if let Some(last) = calls.last_mut() {
            if last.config.is_none() {
                last.config = Some(config);
                return;
            }
        }
        let index = calls.len();
        calls.push(VllmCompileCall {
            index,
            config: Some(config),
            ..Default::default()
        });
    }

    // Add artifact to current (last) compile call's last subgraph,
    // or its pre_subgraph_artifacts if no subgraph exists yet
    pub fn add_artifact(&self, filename: &std::path::Path, suffix: String) {
        let url = filename.to_string_lossy().to_string();
        let name = filename
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| url.clone());

        // Track piecewise split graph file for linking in summary
        if name.starts_with("vllm_piecewise_split_graph") {
            self.ensure_compile_call();
            let mut calls = self.compile_calls.borrow_mut();
            if let Some(call) = calls.last_mut() {
                call.piecewise_graph_file = Some(url.clone());
            }
        }

        let artifact = ArtifactInfo { name, url, suffix };
        self.ensure_compile_call();
        let mut calls = self.compile_calls.borrow_mut();
        if let Some(call) = calls.last_mut() {
            if let Some(last_subgraph) = call.subgraphs.last_mut() {
                last_subgraph.artifacts.push(artifact);
            } else {
                call.pre_subgraph_artifacts.push(artifact);
            }
        }
    }
}

// Group subgraphs by compile range/size for hierarchical display
fn build_compile_range_groups(subgraphs: &[VllmSubgraphInfo]) -> Vec<VllmCompileRangeGroup> {
    use indexmap::IndexMap;

    let mut groups: IndexMap<String, Vec<VllmSubgraphWithArtifacts>> = IndexMap::new();

    for subgraph in subgraphs.iter() {
        let size_or_range = subgraph.size_or_range();
        let (pass_artifacts, artifacts): (Vec<_>, Vec<_>) = subgraph
            .artifacts
            .iter()
            .cloned()
            .partition(|a| a.name.contains("vllm_post_grad."));
        let artifact_count = artifacts.len();
        let pass_artifact_count = pass_artifacts.len();
        let has_pass_artifacts = pass_artifact_count > 0;
        groups
            .entry(size_or_range)
            .or_default()
            .push(VllmSubgraphWithArtifacts {
                submod_name: subgraph.display_submod_name(),
                artifacts,
                artifact_count,
                pass_artifacts,
                pass_artifact_count,
                has_pass_artifacts,
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
fn build_dynamo_artifacts(pre_subgraph_artifacts: &[ArtifactInfo]) -> Vec<ArtifactInfo> {
    let dynamo_names = [
        "dynamo_side_effects",
        "dynamo_output_graph",
        "dynamo_cpp_guards_str",
        "compilation_metrics",
    ];
    pre_subgraph_artifacts
        .iter()
        .filter(|a| dynamo_names.iter().any(|name| a.name.starts_with(name)))
        .cloned()
        .collect()
}

// Get pattern artifacts from pre_subgraph_artifacts
fn build_pattern_artifacts(pre_subgraph_artifacts: &[ArtifactInfo]) -> Vec<ArtifactInfo> {
    pre_subgraph_artifacts
        .iter()
        .filter(|a| a.name.starts_with("vllm_patterns."))
        .cloned()
        .collect()
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
            self.state.push_compile_call(config);
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
// On compile_start: pushes new VllmSubgraphInfo to current compile call's subgraphs.
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
                    self.state.ensure_compile_call();
                    let mut calls = self.state.compile_calls.borrow_mut();
                    if let Some(call) = calls.last_mut() {
                        call.subgraphs.push(subgraph);
                    }
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

// Parses vllm_post_grad.* and before_post_grad_graph graph dumps.
// Diffs each vllm_post_grad dump against the previous one sequentially.
// The first pass diffs against before_post_grad_graph as the baseline.
// The baseline resets when a new compile call starts.
pub struct VllmPostGradPassDiffParser {
    state: Rc<VllmState>,
    /// Last seen payload for diffing consecutive passes
    previous_payload: RefCell<Option<String>>,
    /// Track which compile call index we're in, to reset baseline on new call
    current_compile_call_index: RefCell<Option<usize>>,
    /// Buffered pattern content keyed by pass name (e.g. "RMSNormQuantFusionPass")
    patterns: RefCell<HashMap<String, String>>,
}

impl VllmPostGradPassDiffParser {
    pub fn new(state: Rc<VllmState>) -> Self {
        Self {
            state,
            previous_payload: RefCell::new(None),
            current_compile_call_index: RefCell::new(None),
            patterns: RefCell::new(HashMap::new()),
        }
    }

    /// Check if compile call changed and reset baseline if so
    fn maybe_reset_baseline(&self) {
        let calls = self.state.compile_calls.borrow();
        let current_idx = if calls.is_empty() {
            0
        } else {
            calls.len() - 1
        };
        let mut tracked_idx = self.current_compile_call_index.borrow_mut();
        if *tracked_idx != Some(current_idx) {
            *tracked_idx = Some(current_idx);
            *self.previous_payload.borrow_mut() = None;
        }
    }

    fn generate_diff_html(before: &str, after: &str) -> String {
        use similar::{ChangeTag, TextDiff};

        let diff = TextDiff::from_lines(before, after);

        // Collect changes into side-by-side rows per hunk
        let mut html = String::new();
        let mut has_hunks = false;

        for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
            if !has_hunks {
                html.push_str("<div class=\"diff-wrapper\"><table class=\"diff-table\">\n<colgroup><col class=\"line-num-col\"><col class=\"code-col\"><col class=\"line-num-col\"><col class=\"code-col\"></colgroup>\n");
                has_hunks = true;
            }

            let _ = write!(
                html,
                "<tr class=\"diff-hunk\"><td colspan=\"4\">@@ {} @@</td></tr>\n",
                html_escape::encode_text(&hunk.header().to_string()),
            );

            // Collect changes so we can pair deletions with insertions
            let changes: Vec<_> = hunk.iter_changes().collect();
            let mut i = 0;
            while i < changes.len() {
                match changes[i].tag() {
                    ChangeTag::Equal => {
                        let text = html_escape::encode_text(
                            changes[i].value().trim_end_matches('\n'),
                        );
                        let old_line = changes[i].old_index().map(|n| n + 1);
                        let new_line = changes[i].new_index().map(|n| n + 1);
                        let _ = write!(
                            html,
                            "<tr><td class=\"line-num\">{}</td><td class=\"code-cell\">{}</td><td class=\"line-num right-num\">{}</td><td class=\"code-cell\">{}</td></tr>\n",
                            old_line.map(|n| n.to_string()).unwrap_or_default(),
                            text,
                            new_line.map(|n| n.to_string()).unwrap_or_default(),
                            text,
                        );
                        i += 1;
                    }
                    ChangeTag::Delete => {
                        // Collect consecutive deletes
                        let mut deletes = Vec::new();
                        while i < changes.len() && changes[i].tag() == ChangeTag::Delete {
                            deletes.push(&changes[i]);
                            i += 1;
                        }
                        // Collect consecutive inserts that follow
                        let mut inserts = Vec::new();
                        while i < changes.len() && changes[i].tag() == ChangeTag::Insert {
                            inserts.push(&changes[i]);
                            i += 1;
                        }
                        // Pair them up row by row
                        let max_len = deletes.len().max(inserts.len());
                        for j in 0..max_len {
                            let (left_class, left_num, left_text) = if j < deletes.len() {
                                let num = deletes[j].old_index().map(|n| n + 1);
                                let text = html_escape::encode_text(
                                    deletes[j].value().trim_end_matches('\n'),
                                );
                                (" diff-del", num, text.to_string())
                            } else {
                                ("", None, String::new())
                            };
                            let (right_class, right_num, right_text) = if j < inserts.len() {
                                let num = inserts[j].new_index().map(|n| n + 1);
                                let text = html_escape::encode_text(
                                    inserts[j].value().trim_end_matches('\n'),
                                );
                                (" diff-add", num, text.to_string())
                            } else {
                                ("", None, String::new())
                            };
                            let _ = write!(
                                html,
                                "<tr><td class=\"line-num\">{}</td><td class=\"code-cell{}\">{}</td><td class=\"line-num right-num\">{}</td><td class=\"code-cell{}\">{}</td></tr>\n",
                                left_num.map(|n| n.to_string()).unwrap_or_default(),
                                left_class,
                                left_text,
                                right_num.map(|n| n.to_string()).unwrap_or_default(),
                                right_class,
                                right_text,
                            );
                        }
                    }
                    ChangeTag::Insert => {
                        // Standalone insert (no preceding delete)
                        let text = html_escape::encode_text(
                            changes[i].value().trim_end_matches('\n'),
                        );
                        let new_line = changes[i].new_index().map(|n| n + 1);
                        let _ = write!(
                            html,
                            "<tr><td class=\"line-num\"></td><td class=\"code-cell\"></td><td class=\"line-num right-num\">{}</td><td class=\"code-cell diff-add\">{}</td></tr>\n",
                            new_line.map(|n| n.to_string()).unwrap_or_default(),
                            text,
                        );
                        i += 1;
                    }
                }
            }
        }

        if has_hunks {
            html.push_str("</table></div>\n");
        } else {
            html = "<div class=\"no-changes\">(no changes)</div>\n".to_string();
        }

        html
    }
}

impl StructuredLogParser for VllmPostGradPassDiffParser {
    fn name(&self) -> &'static str {
        "vllm_post_grad_pass_diff"
    }

    fn get_metadata<'e>(&self, e: &'e Envelope) -> Option<Metadata<'e>> {
        if let Some(graph_dump) = &e.graph_dump {
            if graph_dump.name.starts_with("vllm_post_grad.")
                || graph_dump.name.starts_with("vllm_patterns.")
            {
                return Some(Metadata::GraphDump(graph_dump));
            }
        }
        // before_post_grad_graph is logged as an "artifact", not "graph_dump"
        if let Some(artifact) = &e.artifact {
            if artifact.name == "before_post_grad_graph" {
                return Some(Metadata::Artifact(artifact));
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
        // Reset baseline if we moved to a new compile call
        self.maybe_reset_baseline();

        // before_post_grad_graph (artifact): seed baseline for first pass diff
        // Don't output a file — the default ArtifactParser handles that
        if matches!(metadata, Metadata::Artifact(a) if a.name == "before_post_grad_graph") {
            *self.previous_payload.borrow_mut() = Some(payload.to_string());
            return Ok(Vec::new());
        }

        let graph_dump = match metadata {
            Metadata::GraphDump(gd) => gd,
            _ => return Ok(Vec::new()),
        };

        *self.state.has_vllm_artifacts.borrow_mut() = true;

        // Handle vllm_patterns.* graph dumps: buffer and output as .py file
        if let Some(pattern_pass_name) = graph_dump.name.strip_prefix("vllm_patterns.") {
            self.patterns
                .borrow_mut()
                .insert(pattern_pass_name.to_string(), payload.to_string());
            let filename = format!("{}.py", graph_dump.name);
            let f = build_file_path(&filename, lineno, compile_id);
            return Ok(vec![ParserOutput::PayloadFile(f)]);
        }

        let pass_name = graph_dump
            .name
            .strip_prefix("vllm_post_grad.")
            .unwrap_or(&graph_dump.name);

        // Output the post-pass graph as a txt file
        let txt_filename = format!("{}.txt", graph_dump.name);
        let txt_path = build_file_path(&txt_filename, lineno, compile_id);
        let mut results: ParserResults = vec![ParserOutput::PayloadFile(txt_path)];

        // Diff against previous dump and update baseline
        let prev = self.previous_payload.borrow().clone();
        if let Some(before) = prev {
            let diff_html = Self::generate_diff_html(&before, payload);

            // Look up patterns for this pass by stripping the "{digit}." prefix
            let base_pass_name = pass_name
                .find('.')
                .map(|dot| &pass_name[dot + 1..])
                .unwrap_or(pass_name);
            let patterns_map = self.patterns.borrow();
            let patterns_content = patterns_map.get(base_pass_name);
            let has_patterns = patterns_content.is_some();
            let patterns_html = patterns_content
                .map(|p| html_escape::encode_text(p).to_string())
                .unwrap_or_default();

            let context = VllmDiffContext {
                css: super::templates::VLLM_CSS.to_string(),
                pass_name: pass_name.to_string(),
                diff_html,
                patterns_html,
                has_patterns,
                qps: TEMPLATE_QUERY_PARAM_SCRIPT.to_string(),
            };

            let mut tt = TinyTemplate::new();
            tt.add_formatter("format_unescaped", tinytemplate::format_unescaped);
            tt.add_template("vllm_diff.html", super::templates::VLLM_DIFF_TEMPLATE)?;
            let rendered = tt.render("vllm_diff.html", &context)?;

            let diff_filename = format!("vllm_post_grad.{}.diff.html", pass_name);
            let diff_path = build_file_path(&diff_filename, lineno, compile_id);

            results.push(ParserOutput::GlobalFile(diff_path, rendered));
        }

        *self.previous_payload.borrow_mut() = Some(payload.to_string());

        Ok(results)
    }
}

pub fn vllm_parsers_with_state(state: Rc<VllmState>) -> Vec<Box<dyn StructuredLogParser>> {
    vec![
        Box::new(VllmCompilationConfigParser::new(state.clone())),
        Box::new(VllmPiecewiseSplitGraphParser::new(state.clone())),
        Box::new(VllmPiecewiseCompileParser::new(state.clone())),
        Box::new(VllmPostGradPassDiffParser::new(state.clone())),
    ]
}

fn normalize_config(config: &VllmCompilationConfig) -> VllmCompilationConfig {
    let mut config = config.clone();
    if config.compile_ranges_split_points.is_none() {
        if let Some(ref endpoints) = config.compile_ranges_endpoints {
            config.compile_ranges_split_points = Some(endpoints.clone());
        }
    }
    config
}

fn build_call_label(config: &Option<VllmCompilationConfig>) -> String {
    if let Some(cfg) = config {
        if let Some(ref prefix) = cfg.prefix {
            return prefix.clone();
        }
        if let Some(ref model) = cfg.model {
            return model.clone();
        }
    }
    String::new()
}

pub fn generate_vllm_summary(
    state: &VllmState,
    tt: &TinyTemplate,
    custom_header_html: &str,
) -> anyhow::Result<String> {
    let calls = state.compile_calls.borrow();

    // Check if all configs are identical (for shared config display)
    let configs: Vec<_> = calls
        .iter()
        .filter_map(|c| c.config.as_ref())
        .collect();
    let has_shared_config = configs.len() > 1
        && configs.windows(2).all(|w| {
            serde_json::to_string(w[0]).ok() == serde_json::to_string(w[1]).ok()
        });

    let shared_config = if has_shared_config {
        normalize_config(configs[0])
    } else {
        VllmCompilationConfig::default()
    };

    let num_calls = calls.len();
    let has_multiple_calls = num_calls > 1;

    let compile_calls: Vec<VllmCompileCallContext> = calls
        .iter()
        .enumerate()
        .map(|(i, call)| {
            let config = call
                .config
                .as_ref()
                .map(|c| normalize_config(c))
                .unwrap_or_default();
            let has_config = call.config.is_some() && !has_shared_config;
            let dynamo_artifacts = build_dynamo_artifacts(&call.pre_subgraph_artifacts);
            let has_dynamo_artifacts = !dynamo_artifacts.is_empty();
            let pattern_artifacts = build_pattern_artifacts(&call.pre_subgraph_artifacts);
            let has_pattern_artifacts = !pattern_artifacts.is_empty();
            let has_piecewise = call.piecewise_graph_file.is_some();
            let compile_range_groups = build_compile_range_groups(&call.subgraphs);

            VllmCompileCallContext {
                display_index: i + 1,
                label: build_call_label(&call.config),
                is_first: i == 0,
                has_config,
                config,
                dynamo_artifacts,
                has_dynamo_artifacts,
                pattern_artifacts,
                has_pattern_artifacts,
                piecewise_graph_file: call.piecewise_graph_file.clone(),
                has_piecewise,
                compile_range_groups,
            }
        })
        .collect();

    let context = VllmSummaryContext {
        css: super::templates::VLLM_CSS.to_string(),
        qps: TEMPLATE_QUERY_PARAM_SCRIPT.to_string(),
        custom_header_html: custom_header_html.to_string(),
        compile_calls,
        has_multiple_calls,
        has_shared_config,
        shared_config,
    };

    Ok(tt.render("vllm_summary.html", &context)?)
}
