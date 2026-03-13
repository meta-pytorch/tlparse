use crate::parsers::{build_file_path, Metadata, ParserOutput, ParserResults, StructuredLogParser};
use crate::templates::TEMPLATE_QUERY_PARAM_SCRIPT;
use crate::types::{CompileId, Envelope};

use super::types::{
    ArtifactInfo, VllmCompileCall, VllmCompileCallContext, VllmCompileRangeGroup,
    VllmCompilationConfig, VllmDiffContext, VllmSubgraphInfo, VllmSubgraphWithArtifacts,
    VllmSummaryContext,
};

use std::cell::RefCell;
use std::fmt::Write as FmtWrite;
use std::rc::Rc;
use tinytemplate::TinyTemplate;

#[derive(Debug, Default)]
pub struct VllmState {
    pub compile_calls: RefCell<Vec<VllmCompileCall>>,
    pub has_vllm_artifacts: RefCell<bool>,
    /// Buffered compile event type from vllm_compile_event, to be attached to
    /// the next compile call (since the event arrives before vllm_compilation_config).
    pending_compile_event_type: RefCell<Option<String>>,
}

impl VllmState {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn has_artifacts(&self) -> bool {
        *self.has_vllm_artifacts.borrow()
    }

    /// Push a new compile call when a `vllm_compilation_config` is seen.
    /// If the last call has no config yet (created by ensure_compile_call for early artifacts),
    /// populate it instead of creating a new one.
    /// Consumes any pending compile event type from a prior vllm_compile_event.
    pub fn push_compile_call(&self, config: VllmCompilationConfig) {
        let pending_event = self.pending_compile_event_type.borrow_mut().take();
        let mut calls = self.compile_calls.borrow_mut();
        if let Some(last) = calls.last_mut() {
            if last.config.is_none() {
                last.config = Some(config);
                if last.compile_event_type.is_none() {
                    last.compile_event_type = pending_event;
                }
                return;
            }
        }
        let index = calls.len();
        calls.push(VllmCompileCall {
            index,
            config: Some(config),
            compile_event_type: pending_event,
            ..Default::default()
        });
    }

    /// Buffer a compile event type to be attached to the next compile call.
    /// The event always arrives before its corresponding vllm_compilation_config,
    /// so we buffer it until push_compile_call consumes it.
    pub fn set_pending_compile_event(&self, event_type: String) {
        *self.pending_compile_event_type.borrow_mut() = Some(event_type);
    }

    /// Ensure at least one compile call exists. Creates a default one if empty.
    fn ensure_compile_call(&self) {
        let mut calls = self.compile_calls.borrow_mut();
        if calls.is_empty() {
            calls.push(VllmCompileCall::default());
        }
    }

    /// Add artifact to the current (last) compile call's last subgraph,
    /// or to pre_subgraph_artifacts if no subgraph exists yet.
    pub fn add_artifact(&self, filename: &std::path::Path, suffix: String) {
        self.ensure_compile_call();

        let url = filename.to_string_lossy().to_string();
        let name = filename
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| url.clone());

        // Track piecewise split graph file on the current compile call
        if name.starts_with("vllm_piecewise_split_graph") {
            let mut calls = self.compile_calls.borrow_mut();
            if let Some(last) = calls.last_mut() {
                last.piecewise_graph_file = Some(url.clone());
            }
        }

        let artifact = ArtifactInfo { name, url, suffix };
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
fn build_compile_range_groups(call: &VllmCompileCall) -> Vec<VllmCompileRangeGroup> {
    use indexmap::IndexMap;

    let mut groups: IndexMap<String, Vec<VllmSubgraphWithArtifacts>> = IndexMap::new();

    for subgraph in call.subgraphs.iter() {
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
fn build_dynamo_artifacts(call: &VllmCompileCall) -> Vec<ArtifactInfo> {
    let dynamo_names = [
        "dynamo_side_effects",
        "dynamo_output_graph",
        "dynamo_cpp_guards_str",
        "compilation_metrics",
    ];
    call.pre_subgraph_artifacts
        .iter()
        .filter(|a| dynamo_names.iter().any(|name| a.name.starts_with(name)))
        .cloned()
        .collect()
}

// Get pattern artifacts from pre_subgraph_artifacts
fn build_pattern_artifacts(call: &VllmCompileCall) -> Vec<ArtifactInfo> {
    call.pre_subgraph_artifacts
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

// Parses vllm_compile_event artifacts emitted by vLLM's @support_torch_compile decorator.
// Stores the event type (aot_cache_hit / fresh_compile) on the current compile call.
pub struct VllmCompileEventParser {
    state: Rc<VllmState>,
}

impl VllmCompileEventParser {
    pub fn new(state: Rc<VllmState>) -> Self {
        Self { state }
    }
}

#[derive(serde::Deserialize)]
struct VllmCompileEvent {
    #[serde(rename = "type")]
    event_type: String,
}

impl StructuredLogParser for VllmCompileEventParser {
    fn name(&self) -> &'static str {
        "vllm_compile_event"
    }

    fn get_metadata<'e>(&self, e: &'e Envelope) -> Option<Metadata<'e>> {
        if let Some(artifact) = &e.artifact {
            if artifact.name == "vllm_compile_event" {
                return Some(Metadata::Artifact(artifact));
            }
        }
        None
    }

    fn parse<'e>(
        &self,
        _lineno: usize,
        _metadata: Metadata<'e>,
        _rank: Option<u32>,
        _compile_id: &Option<CompileId>,
        payload: &str,
    ) -> anyhow::Result<ParserResults> {
        if let Ok(event) = serde_json::from_str::<VllmCompileEvent>(payload) {
            self.state.set_pending_compile_event(event.event_type);
        }
        Ok(Vec::new())
    }
}

// Parses vllm_piecewise_compile_start artifacts and vllm_subgraph_*/vllm_submod_* graph dumps.
// On compile_start: pushes new VllmSubgraphInfo to the current compile call's subgraphs.
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

// Parses three kinds of log entries to produce per-pass diff pages:
//
//   1. "before_post_grad_graph" artifact — the graph before any passes run.
//      Stored as the diff baseline; no file output (ArtifactParser handles that).
//
//   2. "vllm_patterns.<PassName>" graph dump — pattern matcher source for a pass.
//      Output as a standalone .py file (linked from the summary page).
//
//   3. "vllm_post_grad.<index>.<PassName>" graph dump — the graph after a pass.
//      Diffed against `previous_payload` to produce a side-by-side HTML diff,
//      then becomes the new baseline for the next pass.
pub struct VllmPostGradPassDiffParser {
    state: Rc<VllmState>,
    // The graph payload from the previous pass (or before_post_grad_graph),
    // used as the "before" side of the next diff.
    previous_payload: RefCell<Option<String>>,
    /// Tracks which compile call we're in, to reset baseline on new compile call.
    current_compile_call_index: RefCell<usize>,
}

impl VllmPostGradPassDiffParser {
    pub fn new(state: Rc<VllmState>) -> Self {
        Self {
            state,
            previous_payload: RefCell::new(None),
            current_compile_call_index: RefCell::new(0),
        }
    }

    /// Check if we've moved to a new compile call, and reset baseline if so.
    fn check_compile_call_change(&self) {
        let calls = self.state.compile_calls.borrow();
        let current_idx = if calls.is_empty() { 0 } else { calls.len() - 1 };
        let mut tracked_idx = self.current_compile_call_index.borrow_mut();
        if current_idx != *tracked_idx {
            *tracked_idx = current_idx;
            *self.previous_payload.borrow_mut() = None;
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
        self.check_compile_call_change();

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

        // Handle vllm_patterns.* graph dumps: output as a standalone .py file
        if graph_dump.name.starts_with("vllm_patterns.") {
            let filename = format!("{}.py", graph_dump.name);
            let f = build_file_path(&filename, lineno, compile_id);
            return Ok(vec![ParserOutput::PayloadFile(f)]);
        }

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
        Box::new(VllmCompileEventParser::new(state.clone())),
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
    let calls = state.compile_calls.borrow();

    // Build per-call contexts
    let mut compile_call_contexts: Vec<VllmCompileCallContext> = Vec::new();
    for (i, call) in calls.iter().enumerate() {
        let config = call
            .config
            .as_ref()
            .map(|c| normalize_config(c))
            .unwrap_or_default();
        let has_config = call.config.is_some();
        let dynamo_artifacts = build_dynamo_artifacts(call);
        let has_dynamo_artifacts = !dynamo_artifacts.is_empty();
        let pattern_artifacts = build_pattern_artifacts(call);
        let has_pattern_artifacts = !pattern_artifacts.is_empty();
        let has_piecewise = call.piecewise_graph_file.is_some();
        let compile_range_groups = build_compile_range_groups(call);

        // Label: use config prefix if available
        let label = call
            .config
            .as_ref()
            .and_then(|c| c.prefix.clone())
            .unwrap_or_default();

        let is_cache_hit = call
            .compile_event_type
            .as_deref()
            == Some("aot_cache_hit");

        compile_call_contexts.push(VllmCompileCallContext {
            display_index: i + 1,
            label,
            is_first: i == 0,
            is_cache_hit,
            has_config,
            config,
            dynamo_artifacts,
            has_dynamo_artifacts,
            pattern_artifacts,
            has_pattern_artifacts,
            piecewise_graph_file: call.piecewise_graph_file.clone(),
            has_piecewise,
            compile_range_groups,
        });
    }

    let has_multiple_calls = compile_call_contexts.len() > 1;

    // Detect shared config: if all calls have the same config (by JSON equality)
    let (has_shared_config, shared_config) = if has_multiple_calls {
        let configs: Vec<_> = calls
            .iter()
            .filter_map(|c| c.config.as_ref())
            .collect();
        if configs.len() >= 2 {
            let first_json = serde_json::to_string(&normalize_config(configs[0])).unwrap_or_default();
            let all_same = configs[1..]
                .iter()
                .all(|c| serde_json::to_string(&normalize_config(c)).unwrap_or_default() == first_json);
            if all_same {
                // Hide per-call config since they're all the same
                for ctx in &mut compile_call_contexts {
                    ctx.has_config = false;
                }
                (true, normalize_config(configs[0]))
            } else {
                (false, VllmCompilationConfig::default())
            }
        } else {
            (false, VllmCompilationConfig::default())
        }
    } else {
        (false, VllmCompilationConfig::default())
    };

    let context = VllmSummaryContext {
        css: super::templates::VLLM_CSS.to_string(),
        qps: TEMPLATE_QUERY_PARAM_SCRIPT.to_string(),
        custom_header_html: custom_header_html.to_string(),
        compile_calls: compile_call_contexts,
        has_multiple_calls,
        has_shared_config,
        shared_config,
    };

    Ok(tt.render("vllm_summary.html", &context)?)
}
