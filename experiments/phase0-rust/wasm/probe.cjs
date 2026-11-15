const {parse_json,segment_japanese}=require('./pkg/article_wasm_poc.js');
const source='---\nversion: 1\n---\n# 見出し\n\n- A\n- B\n\n| col | value |\n| --- | ---: |\n| x | 3 |\n';
const result=JSON.parse(parse_json(source));
const chunks=JSON.parse(segment_japanese('記事で使える表示をひととおり並べて描画の崩れを確かめるためのカタログ'));
console.log(JSON.stringify({runtime:typeof Bun==='undefined'?'Node':'Bun',nodes:result.children.map(c=>c.type),segments:chunks,wasmBytes:require('node:fs').statSync('./pkg/article_wasm_poc_bg.wasm').size}));
