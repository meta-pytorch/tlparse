# tlparse: Parse structured PT2 logs
`tlparse` parses structured torch trace logs and outputs HTML files analyzing data.

Quick start:
Run PT2 with the TORCH_TRACE environment variable set:
```
TORCH_TRACE=/tmp/my_traced_log_dir python example.py
```

Feed input into tlparse (this will process the latest log file in the directory):
```
tlparse /tmp/my_traced_log_dir -o tl_out/ --latest
```

You can also specify a log filename:

```
tlparse /tmp/my_traced_log_dir/dedicated_log_torch_trace_ro2i1hvn.log -o tl_out/
```

## Adding custom parsers
You can extend tlparse with custom parsers which take existing structured log data and output any file. To do so, first implement StructuredLogParser with your own trait:

```Rust
pub struct MyCustomParser;
impl StructuredLogParser for MyCustomParser {
    fn name(&self) -> &'static str {
        "my_custom_parser"
    }
    fn get_metadata<'e>(&self, e: &'e Envelope) -> Option<Metadata<'e>> {
        // Get required metadata from the Envelope.
        // You'll need to update Envelope with your custom Metadata if you need new types here
        ....
    }

    fn parse<'e>(&self,
        lineno: usize,
        metadata: Metadata<'e>,
        _rank: Option<u32>,
        compile_id: &Option<CompileId>,
        payload: &str
    ) -> anyhow::Result<ParserResult> {
       // Use the metadata and payload however you'd like
       // Return either a ParserOutput::File(filename, payload) or ParserOutput::Link(name, url)
    }
}
```

## How to release

1. Make a release commit by updating Cargo.toml and then running cargo update
2. Push the release commit and a tag for it.  This will trigger PyPI release
3. cargo publish




## New workflow:

```

# export NVM_DIR="$HOME/.nvm" && [ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"                   
  # 1. Parse logs                    
  cargo run -- <log_file> -o tl_out --no-browser

  # 2. Create standalone viewer (one command)
  ./scripts/make_viewer.sh tl_out

  # 3. Open — no server needed
  open tl_out/viewer.html
      
```