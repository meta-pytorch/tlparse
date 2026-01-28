//! HTML templates for vLLM visualization.

/// CSS for vLLM summary page
pub const VLLM_CSS: &str = r#"
body {
    font-family: Arial, sans-serif;
    margin: 20px;
    background: #f5f5f5;
}
h1 {
    color: #333;
    border-bottom: 2px solid #4a90d9;
    padding-bottom: 10px;
}
h2 {
    color: #4a90d9;
    margin-top: 30px;
}
h3 {
    color: #666;
    margin-top: 20px;
}
.config-table {
    background: white;
    border-collapse: collapse;
    margin: 10px 0;
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}
.config-table td, .config-table th {
    padding: 8px 16px;
    border: 1px solid #ddd;
    text-align: left;
}
.config-table tr:nth-child(even) {
    background: #f9f9f9;
}
.compile-range-group {
    margin: 20px 0;
    padding: 15px;
    border-radius: 8px;
    background: white;
    border: 1px solid #ddd;
}
.compile-range-group h3 {
    margin: 0;
    padding-bottom: 0;
}
.submods-container {
    margin-left: 30px;
    margin-top: 15px;
    padding-left: 15px;
    border-left: 2px solid rgba(0,0,0,0.1);
}
.submods-container > summary {
    cursor: pointer;
    font-weight: 500;
    color: #555;
    padding: 5px 0;
}
.submods-container > summary:hover {
    color: #4a90d9;
}
.submods-container[open] > summary {
    margin-bottom: 10px;
    border-bottom: 1px solid rgba(0,0,0,0.1);
    padding-bottom: 10px;
}
.subgraph {
    background: rgba(255,255,255,0.7);
    padding: 12px 12px 12px 20px;
    margin: 10px 0 10px 25px;
    border-radius: 5px;
    border: 1px solid rgba(0,0,0,0.1);
}
.subgraph h4 {
    margin: 0 0 8px 0;
    color: #333;
    font-size: 0.95em;
}
.subgraph ul {
    margin: 5px 0;
    padding-left: 20px;
}
.subgraph a {
    color: #4a90d9;
    text-decoration: none;
}
.subgraph a:hover {
    text-decoration: underline;
}
.artifact-section {
    margin-top: 10px;
    padding: 10px;
    background: rgba(0, 0, 0, 0.03);
    border-radius: 4px;
}
.artifact-section summary {
    cursor: pointer;
    font-weight: 500;
    color: #666;
}
.artifact-section summary:hover {
    color: #4a90d9;
}
.artifact-list {
    margin: 10px 0 0 0;
    padding-left: 20px;
    list-style-type: disc;
}
.artifact-list li {
    margin: 4px 0;
}
.artifact-list a {
    color: #4a90d9;
    text-decoration: none;
}
.artifact-list a:hover {
    text-decoration: underline;
}
.summary-box {
    background: white;
    padding: 15px;
    margin: 10px 0;
    border-radius: 5px;
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}
.summary-box a {
    color: #4a90d9;
    text-decoration: none;
}
.summary-box a:hover {
    text-decoration: underline;
}
"#;

pub const VLLM_SUMMARY_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>vLLM Compilation Summary</title>
    <style>
{css | format_unescaped}
    </style>
</head>
<body>
{custom_header_html | format_unescaped}
    <div style="background: #e8f4fd; border: 1px solid #4a90d9; border-radius: 5px; padding: 10px 15px; margin-bottom: 20px;">
        This is the vLLM compilation view. <a href="tlparse_index.html">View original tlparse output →</a>
    </div>
    <h1>vLLM Compilation Summary</h1>

    {{ if has_config }}
    <h2>Compilation Configuration</h2>
    <details open>
        <summary><strong>Core Settings</strong></summary>
        <table class="config-table">
            <tr><td><strong>Model</strong></td><td>{config.model}</td></tr>
            <tr><td><strong>Mode</strong></td><td>{config.mode}</td></tr>
            <tr><td><strong>Backend</strong></td><td>{config.backend}</td></tr>
            <tr><td><strong>Prefix</strong></td><td>{config.prefix}</td></tr>
            <tr><td><strong>Custom Ops</strong></td><td>{config.custom_ops}</td></tr>
            <tr><td><strong>Splitting Ops</strong></td><td>{config.splitting_ops}</td></tr>
        </table>
    </details>
    <details open>
        <summary><strong>Compile Settings</strong></summary>
        <table class="config-table">
            <tr><td><strong>CUDAGraph Mode</strong></td><td>{config.cudagraph_mode}</td></tr>
            <tr><td><strong>Use Inductor Graph Partition</strong></td><td>{config.use_inductor_graph_partition}</td></tr>
            <tr><td><strong>Compile Sizes</strong></td><td>{config.compile_sizes}</td></tr>
            <tr><td><strong>Compile Ranges Split Points</strong></td><td>{config.compile_ranges_split_points}</td></tr>
            <tr><td><strong>Inductor Passes</strong></td><td>{config.inductor_passes}</td></tr>
            <tr><td><strong>Enabled Passes</strong></td><td>{config.enabled_passes}</td></tr>
            <tr><td><strong>Dynamic Shapes Type</strong></td><td>{config.dynamic_shapes_type}</td></tr>
            <tr><td><strong>Dynamic Shapes Evaluate Guards</strong></td><td>{config.dynamic_shapes_evaluate_guards}</td></tr>
        </table>
    </details>
    {{ endif }}

    <div class="summary-box">
        <p>PT2 generates <a href="chromium_events.json">Chromium Trace Events</a> in JSON on specific events during compilation.
        You can download and view them in a tool like <a href="https://ui.perfetto.dev/">Perfetto</a>.</p>
    </div>

    {{ if has_dynamo_artifacts }}
    <h2>Dynamo Compilation</h2>
    <div class="summary-box">
        <ul class="artifact-list">
        {{ for artifact in dynamo_artifacts }}
            <li><a href="{artifact.url}">{artifact.name}</a> {artifact.suffix}</li>
        {{ endfor }}
        </ul>
    </div>
    {{ endif }}

    {{ if has_piecewise }}
    <h2>Piecewise Split Graph</h2>
    <div class="summary-box">
        <ul class="artifact-list">
            <li><a href="{piecewise_graph_file}">vllm_piecewise_split_graph</a></li>
        </ul>
    </div>
    {{ endif }}

    <h2>Inductor Compilation</h2>

    {{ for group in compile_range_groups }}
    <div class="compile-range-group">
        <h3>{group.size_or_range}</h3>

        <details open class="submods-container">
            <summary>Subgraphs ({group.submod_count})</summary>
            {{ for subgraph in group.submods }}
            <div class="subgraph">
                <h4>{subgraph.submod_name}</h4>
                {{ if subgraph.artifacts }}
                <div class="artifact-section">
                    <details open>
                        <summary>Artifacts ({subgraph.artifact_count} files)</summary>
                        <ul class="artifact-list">
                        {{ for artifact in subgraph.artifacts }}
                            <li><a href="{artifact.url}">{artifact.name}</a> {artifact.suffix}</li>
                        {{ endfor }}
                        </ul>
                    </details>
                </div>
                {{ endif }}
            </div>
            {{ endfor }}
        </details>
    </div>
    {{ endfor }}
{qps | format_unescaped}
</body>
</html>
"#;
