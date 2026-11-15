use super::{
    CodeHighlighting, CodeRole, CodeToken, PlainCodeReason, PreparationError, RenderCodeBlock,
};
use crate::CodeBlock;
use std::sync::LazyLock;
use syntect::{
    parsing::{ParseState, ScopeStack, SyntaxSet},
    util::LinesWithEndings,
};

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub enum CodeLanguage {
    Bash,
    C,
    Clojure,
    CMake,
    Cpp,
    CSharp,
    Css,
    Dart,
    Diff,
    Dockerfile,
    Elixir,
    Go,
    GraphQl,
    Haskell,
    Html,
    Ini,
    Java,
    JavaScript,
    Json,
    Julia,
    Kotlin,
    Lua,
    Makefile,
    Markdown,
    Perl,
    Php,
    Python,
    R,
    Ruby,
    Rust,
    Scala,
    Scss,
    Sql,
    Swift,
    Terraform,
    Toml,
    TypeScript,
    TypeScriptReact,
    Xml,
    Yaml,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodeLanguageDefinition {
    pub language: CodeLanguage,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub syntax_name: &'static str,
}

pub const PLAIN_LANGUAGE_NAMES: &[&str] = &["text", "plain", "plaintext"];

pub fn code_languages() -> &'static [CodeLanguageDefinition] {
    LANGUAGES
}

pub fn recognize_code_language(name: &str) -> Option<CodeLanguage> {
    LANGUAGES
        .iter()
        .find(|entry| {
            entry.name.eq_ignore_ascii_case(name.trim())
                || entry
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(name.trim()))
        })
        .map(|entry| entry.language)
}

impl CodeLanguage {
    pub fn as_str(self) -> &'static str {
        self.definition().name
    }
    pub fn definition(self) -> &'static CodeLanguageDefinition {
        LANGUAGES
            .iter()
            .find(|entry| entry.language == self)
            .expect("closed language table")
    }
}

// 認識する言語は公開契約として定義し、two-faceのfancy構文セットは全体を同梱する。
const LANGUAGES: &[CodeLanguageDefinition] = &[
    CodeLanguageDefinition {
        language: CodeLanguage::Bash,
        name: "bash",
        aliases: &["sh", "shell", "zsh"],
        syntax_name: "Bourne Again Shell (bash)",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::C,
        name: "c",
        aliases: &[],
        syntax_name: "C",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Clojure,
        name: "clojure",
        aliases: &["clj", "cljs", "cljc"],
        syntax_name: "Clojure",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::CMake,
        name: "cmake",
        aliases: &[],
        syntax_name: "CMake",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Cpp,
        name: "cpp",
        aliases: &["c++", "cxx"],
        syntax_name: "C++",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::CSharp,
        name: "csharp",
        aliases: &["c#", "cs", "csharp"],
        syntax_name: "C#",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Css,
        name: "css",
        aliases: &[],
        syntax_name: "CSS",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Dart,
        name: "dart",
        aliases: &[],
        syntax_name: "Dart",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Diff,
        name: "diff",
        aliases: &["patch"],
        syntax_name: "Diff",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Dockerfile,
        name: "dockerfile",
        aliases: &["docker"],
        syntax_name: "Dockerfile",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Elixir,
        name: "elixir",
        aliases: &["ex", "exs"],
        syntax_name: "Elixir",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Go,
        name: "go",
        aliases: &["golang"],
        syntax_name: "Go",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::GraphQl,
        name: "graphql",
        aliases: &["gql"],
        syntax_name: "GraphQL",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Haskell,
        name: "haskell",
        aliases: &["hs"],
        syntax_name: "Haskell",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Html,
        name: "html",
        aliases: &["htm"],
        syntax_name: "HTML",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Ini,
        name: "ini",
        aliases: &["cfg"],
        syntax_name: "INI",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Java,
        name: "java",
        aliases: &[],
        syntax_name: "Java",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::JavaScript,
        name: "javascript",
        aliases: &["js"],
        syntax_name: "JavaScript",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Json,
        name: "json",
        aliases: &[],
        syntax_name: "JSON",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Julia,
        name: "julia",
        aliases: &["jl"],
        syntax_name: "Julia",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Kotlin,
        name: "kotlin",
        aliases: &["kt", "kts"],
        syntax_name: "Kotlin",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Lua,
        name: "lua",
        aliases: &[],
        syntax_name: "Lua",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Makefile,
        name: "makefile",
        aliases: &["make", "mk"],
        syntax_name: "Makefile",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Markdown,
        name: "markdown",
        aliases: &["md"],
        syntax_name: "Markdown",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Perl,
        name: "perl",
        aliases: &["pl", "pm"],
        syntax_name: "Perl",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Php,
        name: "php",
        aliases: &[],
        syntax_name: "PHP",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Python,
        name: "python",
        aliases: &["py"],
        syntax_name: "Python",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::R,
        name: "r",
        aliases: &[],
        syntax_name: "R",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Ruby,
        name: "ruby",
        aliases: &["rb"],
        syntax_name: "Ruby",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Rust,
        name: "rust",
        aliases: &["rs"],
        syntax_name: "Rust",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Scala,
        name: "scala",
        aliases: &["sc"],
        syntax_name: "Scala",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Scss,
        name: "scss",
        aliases: &[],
        syntax_name: "SCSS",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Sql,
        name: "sql",
        aliases: &[],
        syntax_name: "SQL",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Swift,
        name: "swift",
        aliases: &[],
        syntax_name: "Swift",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Terraform,
        name: "terraform",
        aliases: &["hcl", "tf", "tfvars"],
        syntax_name: "Terraform",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Toml,
        name: "toml",
        aliases: &[],
        syntax_name: "TOML",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::TypeScript,
        name: "typescript",
        aliases: &["ts"],
        syntax_name: "TypeScript",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::TypeScriptReact,
        name: "typescriptreact",
        aliases: &["tsx", "typescript-react"],
        syntax_name: "TypeScriptReact",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Xml,
        name: "xml",
        aliases: &[],
        syntax_name: "XML",
    },
    CodeLanguageDefinition {
        language: CodeLanguage::Yaml,
        name: "yaml",
        aliases: &["yml"],
        syntax_name: "YAML",
    },
];

pub(super) fn prepare_code(code: &CodeBlock) -> Result<RenderCodeBlock, PreparationError> {
    let name = code.language.trim();
    let highlighting = if name.is_empty() {
        plain(PlainCodeReason::NoLanguage)
    } else if PLAIN_LANGUAGE_NAMES
        .iter()
        .any(|plain| plain.eq_ignore_ascii_case(name))
    {
        plain(PlainCodeReason::ExplicitPlain)
    } else if let Some(language) = recognize_code_language(name) {
        let syntax = SYNTAXES
            .find_syntax_by_name(language.definition().syntax_name)
            .ok_or_else(|| PreparationError::MissingSyntax {
                syntax_name: language.definition().syntax_name.to_owned(),
            })?;
        match tokenize(&code.value, ParseState::new(syntax)) {
            Ok(tokens) => CodeHighlighting::Highlighted { language, tokens },
            Err(message) => plain(PlainCodeReason::TokenizationFailed { message }),
        }
    } else {
        plain(PlainCodeReason::UnknownLanguage)
    };
    Ok(RenderCodeBlock {
        id: code.id.clone(),
        language: code.language.clone(),
        value: code.value.clone(),
        highlighting,
    })
}

fn plain(reason: PlainCodeReason) -> CodeHighlighting {
    CodeHighlighting::Plain { reason }
}

fn tokenize(source: &str, mut parser: ParseState) -> Result<Vec<CodeToken>, String> {
    let mut stack = ScopeStack::new();
    let mut tokens = Vec::new();
    let mut base = 0;
    for original in LinesWithEndings::from(source) {
        // extra_newlinesの行末規則を使うための仮想改行は、本文やバイト範囲には含めない。
        let extended;
        let line = if original.ends_with('\n') {
            original
        } else {
            extended = format!("{original}\n");
            &extended
        };
        // TODO(tokenization-backend-errors): regex-fancyの検索エラーはsyntect内部で不一致になるため、plainへの伝播経路を検討する。
        let operations = parser
            .parse_line(line, &SYNTAXES)
            .map_err(|error| error.to_string())?;
        let mut start = 0;
        for (index, operation) in &operations {
            if *index < start || *index > line.len() || !line.is_char_boundary(*index) {
                return Err("Tokenizer produced an invalid UTF-8 byte boundary".to_owned());
            }
            push_token(
                &mut tokens,
                base + start.min(original.len()),
                base + (*index).min(original.len()),
                role(&stack),
            );
            stack.apply(operation).map_err(|error| error.to_string())?;
            start = *index;
        }
        push_token(
            &mut tokens,
            base + start.min(original.len()),
            base + original.len(),
            role(&stack),
        );
        base += original.len();
    }
    Ok(tokens)
}

fn push_token(tokens: &mut Vec<CodeToken>, start: usize, end: usize, role: Option<CodeRole>) {
    if start == end {
        return;
    }
    if let Some(role) = role {
        if let Some(previous) = tokens.last_mut()
            && previous.end == start
            && previous.role == role
        {
            previous.end = end;
        } else {
            tokens.push(CodeToken { start, end, role });
        }
    }
}

fn role(stack: &ScopeStack) -> Option<CodeRole> {
    let role = stack
        .as_slice()
        .iter()
        .rev()
        .find_map(|scope| scope_role(&scope.build_string()));
    if role == Some(CodeRole::String)
        && stack.as_slice().iter().any(|scope| {
            let name = scope.build_string();
            ["meta.mapping.key", "meta.object-literal.key"]
                .iter()
                .any(|prefix| {
                    name == *prefix
                        || name
                            .strip_prefix(prefix)
                            .is_some_and(|rest| rest.starts_with('.'))
                })
        })
    {
        Some(CodeRole::Property)
    } else {
        role
    }
}

fn scope_role(scope: &str) -> Option<CodeRole> {
    let starts = |prefix: &str| {
        scope == prefix
            || scope
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('.'))
    };
    if starts("comment") || starts("punctuation.definition.comment") {
        Some(CodeRole::Comment)
    } else if starts("string")
        || starts("punctuation.definition.string")
        || starts("constant.character.escape")
    {
        Some(CodeRole::String)
    } else if starts("entity.name.function")
        || starts("support.function")
        || starts("variable.function")
    {
        Some(CodeRole::Function)
    } else if starts("entity.name.type")
        || starts("entity.name.class")
        || starts("entity.name.struct")
        || starts("entity.name.enum")
        || starts("entity.name.trait")
        || starts("entity.name.union")
        || starts("support.type")
        || starts("support.class")
    {
        Some(CodeRole::Type)
    } else if starts("variable.other.property")
        || starts("variable.other.member")
        || starts("support.variable.property")
        || starts("entity.other.attribute-name")
    {
        Some(CodeRole::Property)
    } else if starts("keyword.operator") {
        Some(CodeRole::Punctuation)
    } else if starts("keyword") || starts("storage") {
        Some(CodeRole::Keyword)
    } else if starts("variable") {
        Some(CodeRole::Variable)
    } else if starts("punctuation") {
        Some(CodeRole::Punctuation)
    } else {
        None
    }
}
