use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct VllmCompilationConfig {
    pub model: Option<String>,
    pub prefix: Option<String>,
    pub mode: Option<String>,
    pub backend: Option<String>,
    pub custom_ops: Option<String>,
    pub splitting_ops: Option<String>,
    pub cudagraph_mode: Option<String>,
    pub compile_sizes: Option<String>,
    pub compile_ranges_split_points: Option<String>,
    pub use_inductor_graph_partition: Option<bool>,
    pub inductor_passes: Option<String>,
    pub enabled_passes: Option<String>,
    pub dynamic_shapes_type: Option<String>,
    pub dynamic_shapes_evaluate_guards: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VllmSubgraphInfo {
    #[serde(rename = "piecewise_index")]
    pub index: i32,
    #[serde(default)]
    pub submod_name: Option<String>,
    pub compile_range_start: i64,
    pub compile_range_end: i64,
    pub is_single_size: bool,
    #[serde(rename = "is_cudagraph_capture_size")]
    pub is_cudagraph_size: bool,
    #[serde(skip)]
    pub artifacts: Vec<ArtifactInfo>,
}

impl VllmSubgraphInfo {
    pub fn size_or_range(&self) -> String {
        if self.is_single_size {
            format!("size {}", self.compile_range_start)
        } else {
            format!(
                "range [{}, {}]",
                self.compile_range_start, self.compile_range_end
            )
        }
    }

    pub fn display_submod_name(&self) -> String {
        self.submod_name
            .clone()
            .unwrap_or_else(|| format!("subgraph_{}", self.index))
    }
}

#[derive(Debug, Serialize)]
pub struct VllmSummaryContext {
    pub css: String,
    pub qps: String,
    pub custom_header_html: String,
    pub config: VllmCompilationConfig,
    pub has_config: bool,
    pub dynamo_artifacts: Vec<ArtifactInfo>,
    pub has_dynamo_artifacts: bool,
    pub piecewise_graph_file: Option<String>,
    pub has_piecewise: bool,
    pub compile_range_groups: Vec<VllmCompileRangeGroup>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VllmSubgraphWithArtifacts {
    pub submod_name: String,
    pub artifacts: Vec<ArtifactInfo>,
    pub artifact_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct VllmCompileRangeGroup {
    pub size_or_range: String,
    pub submod_count: usize,
    pub submods: Vec<VllmSubgraphWithArtifacts>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactInfo {
    pub name: String,
    pub url: String,
    pub suffix: String,
}
