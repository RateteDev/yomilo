use markdown::{to_mdast, ParseOptions};
use two_face::re_exports::syntect::parsing::{ParseState, ScopeStack};

fn main() {
    let sample = "---\nversion: 1\n---\n# 見出し\n\n段落の**強調**と[リンク](https://example.com)。\n\n- リスト\n\n| 項目 | 値 |\n| --- | ---: |\n| A | 42 |\n\n```typescript\nconst x: number = 1;\n```\n";
    let mut opt = ParseOptions::gfm();
    opt.constructs.frontmatter = true;
    let tree = to_mdast(sample, &opt).unwrap();
    let json = serde_json::to_value(&tree).unwrap();
    let kinds: Vec<_> = json["children"].as_array().unwrap().iter().map(|v| v["type"].as_str().unwrap_or("?" )).collect();
    println!("MDAST basic: {kinds:?}");
    println!("MDAST code language: {}", json["children"].as_array().unwrap().last().unwrap()["lang"]);
    let unsupported = "# test\n\n- [x] task\n\nFootnote[^1] and ~~strike~~.\n\n[^1]: footnote\n";
    let tree2 = to_mdast(unsupported, &opt).unwrap();
    let js2 = serde_json::to_value(&tree2).unwrap();
    println!("MDAST disallowed recognized: {}", js2.to_string().contains("taskListItem") || js2.to_string().contains("checked"));
    println!("MDAST disallowed types contains delete / footnote: {}, {}",js2.to_string().contains("delete"),js2.to_string().contains("footnote"));
    let directive = "# demo\n\n:::callout{type=note label=補足}\n中身はMarkdown。\n\n- 箇条書きも書ける\n:::\n\n::image{src=figure.png alt=説明 ratio=16/9 zoom}\n";
    let j3 = serde_json::to_value(to_mdast(directive, &opt).unwrap()).unwrap();
    println!("MDAST directive raw: {}", j3.to_string().chars().take(1300).collect::<String>());
    let catalog_path = std::env::args().nth(1).expect("usage: native <existing-article.mdx>");
    let catalog = std::fs::read_to_string(catalog_path).unwrap();
    match to_mdast(&catalog, &opt) {
       Ok(tree) => { let j=serde_json::to_value(tree).unwrap(); println!("Existing catalog: parsed; top-level nodes={}",j["children"].as_array().map(|a|a.len()).unwrap_or_default()); },
       Err(e) => println!("Existing catalog: parse failed: {}", e),
    }
    let syn_set = two_face::syntax::extra_newlines();
    for name in ["typescript", "tsx", "jsx", "jsonc", "sql", "dockerfile", "bash", "rust", "python", "dart", "kotlin", "toml", "go", "yaml"] {
       let syntax = syn_set.find_syntax_by_token(name).or_else(|| syn_set.find_syntax_by_extension(name));
       println!("Syntax {name}: {}",syntax.map(|s| s.name.as_str()).unwrap_or("MISSING"));
    }
    if let Some(s) = syn_set.find_syntax_by_extension("ts") {
        let mut state = ParseState::new(s);
        let operations=state.parse_line("const name: string = 'hello';\n",&syn_set).unwrap();
        let mut stack=ScopeStack::new();
        for (offset,operation) in &operations {
          stack.apply(operation).unwrap();
          println!("TS scope at {offset}: {}",stack.scopes.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "));
        }
    }
    let parser=budouy::model::load_default_japanese_parser();
    let phrase="記事で使える表示をひととおり並べて描画の崩れを確かめるためのカタログ";
    let chunks=parser.parse(phrase);
    println!("Budouy sample: {:?}",chunks);
}
