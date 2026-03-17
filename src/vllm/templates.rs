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

pub const VLLM_DIFF_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Pass Diff: {pass_name}</title>
    <style>
{css | format_unescaped}
.diff-wrapper \{
    overflow-x: auto;
    margin: 15px 0;
    border: 1px solid #d0d7de;
    border-radius: 5px;
}
.diff-table \{
    font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
    font-size: 13px;
    line-height: 1.4;
    border-collapse: collapse;
    table-layout: fixed;
    width: 100%;
    background: white;
}
.diff-table col.line-num-col \{
    width: 40px;
}
.diff-table col.code-col \{
    width: calc(50% - 40px);
}
.diff-table td \{
    white-space: pre-wrap;
    word-break: break-all;
    padding: 1px 8px;
    vertical-align: top;
}
.diff-table .line-num \{
    white-space: nowrap;
    text-align: right;
    color: #8b949e;
    padding: 1px 4px;
    user-select: none;
    overflow: visible;
}
.diff-table .right-num \{
    border-left: 1px solid #d0d7de;
}
.diff-add \{
    background: #e6ffec;
}
.diff-del \{
    background: #ffebe9;
}
.diff-hunk \{
    background: #ddf4ff;
}
.diff-hunk td \{
    color: #0969da;
    font-weight: bold;
    padding: 5px 8px;
}
.no-changes \{
    padding: 20px;
    text-align: center;
    color: #57606a;
}
    </style>
</head>
<body>
    <h1>Pass Diff: {pass_name}</h1>
{diff_html | format_unescaped}
{qps | format_unescaped}
</body>
</html>
"#;

pub const VLLM_SUMMARY_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>vLLM Compilation Summary</title>
    <style>
{css | format_unescaped}
.compile-call-section \{
    margin: 20px 0;
    border: 1px solid #ddd;
    border-radius: 8px;
    background: #fafafa;
\}
.compile-call-section > summary \{
    cursor: pointer;
    padding: 12px 16px;
    list-style: none;
    border-radius: 8px;
    display: flex;
    align-items: center;
\}
.compile-call-section > summary::-webkit-details-marker \{
    display: none;
\}
.compile-call-section > summary::before \{
    content: "▶ ";
    font-size: 0.8em;
    margin-right: 4px;
\}
.compile-call-section[open] > summary::before \{
    content: "▼ ";
\}
.compile-call-section > summary:hover \{
    background: #e8f4fd;
\}
.compile-call-section > summary h2 \{
    display: inline;
    margin: 0;
    font-size: 1.2em;
\}
.compile-call-body \{
    padding: 0 20px 20px 20px;
\}
    </style>
</head>
<body>
{custom_header_html | format_unescaped}
    <div style="background: #e8f4fd; border: 1px solid #4a90d9; border-radius: 5px; padding: 10px 15px; margin-bottom: 20px;">
        This is the vLLM compilation view. <a href="tlparse_index.html">View original tlparse output →</a>
    </div>
    <h1>vLLM Compilation Summary</h1>

    {{ if has_shared_config }}
    <h2>Compilation Configuration (shared)</h2>
    <details open>
        <summary><strong>Core Settings</strong></summary>
        <table class="config-table">
            <tr><td><strong>Model</strong></td><td>{shared_config.model}</td></tr>
            <tr><td><strong>Mode</strong></td><td>{shared_config.mode}</td></tr>
            <tr><td><strong>Backend</strong></td><td>{shared_config.backend}</td></tr>
            <tr><td><strong>Prefix</strong></td><td>{shared_config.prefix}</td></tr>
            <tr><td><strong>Custom Ops</strong></td><td>{shared_config.custom_ops}</td></tr>
            <tr><td><strong>Splitting Ops</strong></td><td>{shared_config.splitting_ops}</td></tr>
        </table>
    </details>
    <details open>
        <summary><strong>Compile Settings</strong></summary>
        <table class="config-table">
            <tr><td><strong>CUDAGraph Mode</strong></td><td>{shared_config.cudagraph_mode}</td></tr>
            <tr><td><strong>Use Inductor Graph Partition</strong></td><td>{shared_config.use_inductor_graph_partition}</td></tr>
            <tr><td><strong>Compile Sizes</strong></td><td>{shared_config.compile_sizes}</td></tr>
            <tr><td><strong>Compile Ranges Endpoints</strong></td><td>{shared_config.compile_ranges_split_points}</td></tr>
            <tr><td><strong>Inductor Passes</strong></td><td>{shared_config.inductor_passes}</td></tr>
            <tr><td><strong>Enabled Passes</strong></td><td>{shared_config.enabled_passes}</td></tr>
            <tr><td><strong>Dynamic Shapes Type</strong></td><td>{shared_config.dynamic_shapes_type}</td></tr>
            <tr><td><strong>Dynamic Shapes Evaluate Guards</strong></td><td>{shared_config.dynamic_shapes_evaluate_guards}</td></tr>
        </table>
    </details>
    {{ endif }}

    <div class="summary-box">
        <p>PT2 generates <a href="chromium_events.json">Chromium Trace Events</a> in JSON on specific events during compilation.
        You can download and view them in a tool like <a href="https://ui.perfetto.dev/">Perfetto</a>.</p>
    </div>

    {{ for call in compile_calls }}
    {{ if has_multiple_calls }}
    <details class="compile-call-section" {{ if call.is_first }}open{{ endif }}>
        <summary><h2>Compile Call {call.display_index}{{ if call.label }}: {call.label}{{ endif }}{{ if call.is_cache_hit }} &#x2705;{{ endif }}</h2></summary>
        <div class="compile-call-body">
    {{ endif }}

    {{ if call.has_config }}
    <h2>Compilation Configuration</h2>
    <details open>
        <summary><strong>Core Settings</strong></summary>
        <table class="config-table">
            <tr><td><strong>Model</strong></td><td>{call.config.model}</td></tr>
            <tr><td><strong>Mode</strong></td><td>{call.config.mode}</td></tr>
            <tr><td><strong>Backend</strong></td><td>{call.config.backend}</td></tr>
            <tr><td><strong>Prefix</strong></td><td>{call.config.prefix}</td></tr>
            <tr><td><strong>Custom Ops</strong></td><td>{call.config.custom_ops}</td></tr>
            <tr><td><strong>Splitting Ops</strong></td><td>{call.config.splitting_ops}</td></tr>
        </table>
    </details>
    <details open>
        <summary><strong>Compile Settings</strong></summary>
        <table class="config-table">
            <tr><td><strong>CUDAGraph Mode</strong></td><td>{call.config.cudagraph_mode}</td></tr>
            <tr><td><strong>Use Inductor Graph Partition</strong></td><td>{call.config.use_inductor_graph_partition}</td></tr>
            <tr><td><strong>Compile Sizes</strong></td><td>{call.config.compile_sizes}</td></tr>
            <tr><td><strong>Compile Ranges Endpoints</strong></td><td>{call.config.compile_ranges_split_points}</td></tr>
            <tr><td><strong>Inductor Passes</strong></td><td>{call.config.inductor_passes}</td></tr>
            <tr><td><strong>Enabled Passes</strong></td><td>{call.config.enabled_passes}</td></tr>
            <tr><td><strong>Dynamic Shapes Type</strong></td><td>{call.config.dynamic_shapes_type}</td></tr>
            <tr><td><strong>Dynamic Shapes Evaluate Guards</strong></td><td>{call.config.dynamic_shapes_evaluate_guards}</td></tr>
        </table>
    </details>
    {{ endif }}

    {{ if call.has_dynamo_artifacts }}
    <h2>Dynamo Compilation</h2>
    <div class="summary-box">
        <ul class="artifact-list">
        {{ for artifact in call.dynamo_artifacts }}
            <li><a href="{artifact.url}">{artifact.name}</a> {artifact.suffix}</li>
        {{ endfor }}
        </ul>
    </div>
    {{ endif }}

    {{ if call.has_piecewise }}
    <h2>Piecewise Split Graph</h2>
    <div class="summary-box">
        <ul class="artifact-list">
            <li><a href="{call.piecewise_graph_file}">vllm_piecewise_split_graph</a></li>
        </ul>
    </div>
    {{ endif }}

    {{ if call.has_pattern_artifacts }}
    <h2>Inductor Pass Patterns</h2>
    <div class="summary-box">
        <ul class="artifact-list">
        {{ for artifact in call.pattern_artifacts }}
            <li><a href="{artifact.url}">{artifact.name}</a> {artifact.suffix}</li>
        {{ endfor }}
        </ul>
    </div>
    {{ endif }}

    {{ if call.compile_range_groups }}
    <h2>Inductor Compilation</h2>

    {{ for group in call.compile_range_groups }}
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
                {{ if subgraph.has_pass_artifacts }}
                <div class="artifact-section">
                    <details>
                        <summary>Inductor Pass Graphs &amp; Diffs ({subgraph.pass_artifact_count} files)</summary>
                        <ul class="artifact-list">
                        {{ for artifact in subgraph.pass_artifacts }}
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
    {{ endif }}

    {{ if has_multiple_calls }}
        </div>
    </details>
    {{ endif }}
    {{ endfor }}
{qps | format_unescaped}
</body>
</html>
"#;
