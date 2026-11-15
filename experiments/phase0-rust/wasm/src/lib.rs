use wasm_bindgen::prelude::*;
use markdown::{to_mdast, ParseOptions};

#[wasm_bindgen]
pub fn parse_json(source: &str) -> String {
  let mut options = ParseOptions::gfm();
  options.constructs.frontmatter = true;
  match to_mdast(source, &options) {
    Ok(tree) => serde_json::to_string(&tree).unwrap(),
    Err(err) => format!("{{\"error\":{:?}}}", err.to_string()),
  }
}

#[wasm_bindgen]
pub fn segment_japanese(text: &str) -> String {
  let parser = budouy::model::load_default_japanese_parser();
  serde_json::to_string(&parser.parse(text)).unwrap()
}

#[wasm_bindgen]
pub fn syntax_probe() -> String {
  use two_face::re_exports::syntect::parsing::{ParseState, ScopeStack};
  let set = two_face::syntax::extra_newlines();
  let known = ["typescript", "tsx", "jsx", "jsonc", "sql", "dockerfile", "kotlin", "dart"]
    .iter().map(|name| {
      let result = set.find_syntax_by_token(name).or_else(|| set.find_syntax_by_extension(name));
      (name.to_string(), result.map(|r| r.name.clone()))
    }).collect::<Vec<_>>();
  let s = set.find_syntax_by_extension("ts").unwrap();
  let mut state = ParseState::new(s);
  let changes = state.parse_line("const n: string = 'hello';\n", &set).unwrap();
  let mut stack = ScopeStack::new();
  let mut samples = Vec::new();
  for (index, operation) in changes {
     stack.apply(&operation).unwrap();
     samples.push((index,stack.scopes.iter().map(|s| s.to_string()).collect::<Vec<_>>()));
  }
  serde_json::json!({"languages":known,"tsScopes":samples}).to_string()
}
