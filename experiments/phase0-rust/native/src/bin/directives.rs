use markdown::{to_mdast, ParseOptions};
use serde_json::Value;
fn md(s: &str) -> Value {
    serde_json::to_value(to_mdast(s, &ParseOptions::gfm()).unwrap()).unwrap()
}
fn is_open(s: &str) -> Option<&str> {
    let s = s.trim_end();
    if s.starts_with(":::callout{") && s.ends_with('}') {return Some("callout");}
    if s.starts_with(":::code-group") && s==":::code-group" {return Some("code-group");}
    if s.starts_with(":::svg{") && s.ends_with('}') {return Some("svg");}
    None
}
fn is_self(s: &str)-> bool {
    ["::image{","::video{","::url-card{","::tweet-card{"].iter().any(|v|s.starts_with(v)&&s.trim_end().ends_with('}'))
}
fn fence(line: &str, active: &mut Option<(char,usize)>) {
    let trimmed=line.trim_start_matches(' ');
    if line.len()-trimmed.len()>3{return;}
    let marker=trimmed.chars().next().unwrap_or(' ');
    if marker!='`' && marker!='~'{return;}
    let n=trimmed.chars().take_while(|v|*v==marker).count();
    if n<3{return;}
    let rest=&trimmed[n..];
    match *active {
        None if marker!='`' || !rest.contains('`')=>*active=Some((marker,n)),
        Some((old,size)) if old==marker && n>=size && rest.trim().is_empty()=>*active=None,
        _=>{}
    }
}
fn parse_parts(source: &str)->Result<Vec<String>,String>{
    let lines:Vec<_>=source.split_inclusive('\n').collect();
    let (mut out,mut i,mut normal,mut active)=(vec![],0,String::new(),None);
    while i<lines.len(){
      let line=lines[i];
      if active.is_none() {
        if let Some(kind)=is_open(line) {
          if !normal.is_empty(){out.push(format!("markdown:{}",md(&normal)["children"].as_array().unwrap().len()));normal.clear();}
          let mut body=String::new();let mut inner=None;let start=i+1;let mut closed=false;
          i+=1;
          while i<lines.len(){
            if inner.is_none() && lines[i].trim()==":::"{closed=true;break;}
            fence(lines[i],&mut inner);body.push_str(lines[i]);i+=1;
          }
          if !closed{return Err(format!("unclosed {kind} at line {start}"));}
          if kind=="svg"{out.push("svg:raw".into());}
          else{let nodes=&md(&body)["children"];out.push(format!("{kind}:{}",nodes.as_array().unwrap().iter().map(|x|x["type"].as_str().unwrap_or("?")).collect::<Vec<_>>().join("+")));}
          i+=1;continue;
        }
        if is_self(line){out.push("self-directive".into());i+=1;continue;}
      }
      fence(line,&mut active);normal.push_str(line);i+=1;
    }
    if !normal.is_empty(){out.push(format!("markdown:{}",md(&normal)["children"].as_array().unwrap().len()));}
    Ok(out)
}
fn main(){
 let cases=[
  ("callout-inner-markdown","# Title\n\n:::callout{type=note}\n補足の**強調**\n\n- one\n- two\n:::\n\nAfter\n"),
  ("self-image","::image{src=figure.png alt=説明 ratio=16/9 zoom}\n"),
  ("code-fence-in-group",":::code-group\n```bash npm\necho ':::'\n:::\n```\n:::\n"),
  ("literal-marker-in-code","```md\n:::callout{type=note}\n::image{src=demo.png}\n```\n"),
  ("svg-raw",":::svg{alt=説明}\n<svg viewBox=\"0 0 100 50\"></svg>\n:::\n"),
  ("missing-close",":::callout{type=note}\n本文\n"),
 ];
 for (name,src) in cases {println!("{} => {:?}",name,parse_parts(src));}
}
