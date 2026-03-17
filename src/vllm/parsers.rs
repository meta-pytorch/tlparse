use crate::parsers::{build_file_path, Metadata, ParserOutput, ParserResults, StructuredLogParser};
use crate::templates::TEMPLATE_QUERY_PARAM_SCRIPT;
use crate::types::{CompileId, Envelope};

use super::types::{
    ArtifactInfo, VllmCompilationConfig, VllmCompileRangeGroup, VllmDiffContext, VllmSubgraphInfo,
    VllmSubgraphWithArtifacts, VllmSummaryContext,
};

use std::cell::RefCell;
use std::fmt::Write as FmtWrite;
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

// Parses two kinds of log entries to produce per-pass diff pages:
//
//   1. "before_post_grad_graph" artifact — the graph before any passes run.
//      Stored as the diff baseline; no file output (ArtifactParser handles that).
//
//   2. "vllm_post_grad.<index>.<PassName>" graph dump — the graph after a pass.
//      Diffed against `previous_payload` to produce a side-by-side HTML diff,
//      then becomes the new baseline for the next pass.
pub struct VllmPostGradPassDiffParser {
    state: Rc<VllmState>,
    // The graph payload from the previous pass (or before_post_grad_graph),
    // used as the "before" side of the next diff.
    previous_payload: RefCell<Option<String>>,
}

impl VllmPostGradPassDiffParser {
    pub fn new(state: Rc<VllmState>) -> Self {
        Self {
            state,
            previous_payload: RefCell::new(None),
        }
    }

    // Build a side-by-side diff HTML table from two text payloads.
    //
    // Uses `similar` to compute a unified diff, then renders each hunk as a
    // 4-column table: [old line num | old code | new line num | new code].
    //
    // For changed regions, consecutive Delete and Insert lines are collected
    // and paired row-by-row so additions appear next to the deletions they
    // replaced (like GitHub's side-by-side view).
    fn generate_diff_html(before: &str, after: &str) -> String {
        use similar::{ChangeTag, TextDiff};

        let diff = TextDiff::from_lines(before, after);

        let mut html = String::new();
        let mut has_hunks = false;

        for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
            // Emit table header on first hunk
            if !has_hunks {
                html.push_str("<div class=\"diff-wrapper\"><table class=\"diff-table\">\n<colgroup><col class=\"line-num-col\"><col class=\"code-col\"><col class=\"line-num-col\"><col class=\"code-col\"></colgroup>\n");
                has_hunks = true;
            }

            let _ = write!(
                html,
                "<tr class=\"diff-hunk\"><td colspan=\"4\">@@ {} @@</td></tr>\n",
                html_escape::encode_text(&hunk.header().to_string()),
            );

            // Walk changes, grouping consecutive deletes+inserts for side-by-side pairing
            let changes: Vec<_> = hunk.iter_changes().collect();
            let mut i = 0;
            while i < changes.len() {
                match changes[i].tag() {
                    // Unchanged context line — show on both sides
                    ChangeTag::Equal => {
                        let text =
                            html_escape::encode_text(changes[i].value().trim_end_matches('\n'));
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
                    // Deletion — collect consecutive deletes, then consecutive inserts,
                    // and pair them row-by-row (excess on either side gets blank cells)
                    ChangeTag::Delete => {
                        let mut deletes = Vec::new();
                        while i < changes.len() && changes[i].tag() == ChangeTag::Delete {
                            deletes.push(&changes[i]);
                            i += 1;
                        }
                        let mut inserts = Vec::new();
                        while i < changes.len() && changes[i].tag() == ChangeTag::Insert {
                            inserts.push(&changes[i]);
                            i += 1;
                        }
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
                    // Standalone insert (not preceded by a delete)
                    ChangeTag::Insert => {
                        let text =
                            html_escape::encode_text(changes[i].value().trim_end_matches('\n'));
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
            if graph_dump.name.starts_with("vllm_post_grad.") {
                return Some(Metadata::GraphDump(graph_dump));
            }
        }
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
        // before_post_grad_graph (artifact): seed baseline for first pass diff.
        // Don't output a file — the default ArtifactParser handles that.
        if matches!(metadata, Metadata::Artifact(a) if a.name == "before_post_grad_graph") {
            *self.previous_payload.borrow_mut() = Some(payload.to_string());
            return Ok(Vec::new());
        }

        let graph_dump = match metadata {
            Metadata::GraphDump(gd) => gd,
            _ => return Ok(Vec::new()),
        };

        *self.state.has_vllm_artifacts.borrow_mut() = true;

        // e.g. "vllm_post_grad.0.FusionPass" -> pass_name = "0.FusionPass"
        let pass_name = graph_dump
            .name
            .strip_prefix("vllm_post_grad.")
            .unwrap_or(&graph_dump.name);

        // Always output the raw post-pass graph as a .txt file
        let txt_filename = format!("{}.txt", graph_dump.name);
        let txt_path = build_file_path(&txt_filename, lineno, compile_id);
        let mut results: ParserResults = vec![ParserOutput::PayloadFile(txt_path)];

        // If we have a baseline, generate a side-by-side diff page
        let prev = self.previous_payload.borrow().clone();
        if let Some(before) = prev {
            let diff_html = Self::generate_diff_html(&before, payload);

            let context = VllmDiffContext {
                css: super::templates::VLLM_CSS.to_string(),
                pass_name: pass_name.to_string(),
                diff_html,
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

        // This pass's output becomes the baseline for the next pass's diff
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

pub fn generate_vllm_summary(
    state: &VllmState,
    tt: &TinyTemplate,
    custom_header_html: &str,
) -> anyhow::Result<String> {
    let config = state
        .config
        .borrow()
        .as_ref()
        .map(|c| normalize_config(c))
        .unwrap_or_default();
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
