//! Best-effort parsing of LeetCode problem HTML: description → doc text, and the worked
//! examples → runnable test assertions (falling back to fill-in-by-hand stubs).

use std::sync::LazyLock;

use ego_tree::NodeRef;
use regex::Regex;
use scraper::{Html, Node, Selector};

use crate::harness::fetch::MetaData;

const WRAP_WIDTH: usize = 100;

/// An `Input:` / `Output:` pair lifted verbatim from the problem statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawExample {
    pub input: String,
    pub output: String,
}

/// One generated test: either a real assertion or an `#[ignore]`d stub to translate by hand.
#[derive(Debug, PartialEq, Eq)]
pub enum ExampleTest {
    Assert {
        /// Rust expressions, one per solution parameter.
        args: Vec<String>,
        /// Rust expression for the expected value.
        expected: String,
        /// Compare with an epsilon instead of `assert_eq!` (double-returning problems).
        approx: bool,
    },
    Stub {
        input: String,
        output: String,
    },
}

/// Convert problem-statement HTML into rustdoc-friendly plain text: paragraphs wrapped at
/// ~100 columns, `<pre>` blocks as ```` ```text ```` fences, lists as bullets.
pub fn html_to_doc_text(html: &str) -> String {
    let doc = Html::parse_fragment(html);
    let mut blocks = Vec::new();
    render_blocks(doc.tree.root(), &mut blocks);
    blocks.join("\n\n")
}

/// Inline elements that flow within a paragraph even when they appear between block elements.
const INLINE_ELEMENTS: &[&str] = &[
    "code", "sup", "sub", "strong", "b", "em", "i", "u", "span", "a", "br", "img", "small", "font",
];

fn render_blocks(node: NodeRef<'_, Node>, blocks: &mut Vec<String>) {
    let mut pending = String::new();
    for child in node.children() {
        match child.value() {
            Node::Element(element) => match element.name() {
                "p" => {
                    flush_pending(&mut pending, blocks);
                    let text = inline_text(child);
                    let text = text.trim();
                    if !text.is_empty() {
                        blocks.push(wrap_text(text));
                    }
                }
                "pre" => {
                    flush_pending(&mut pending, blocks);
                    let text = raw_text(child);
                    blocks.push(format!("```text\n{}\n```", text.trim_matches('\n')));
                }
                "ul" | "ol" => {
                    flush_pending(&mut pending, blocks);
                    let items: Vec<String> = child
                        .children()
                        .filter(|li| matches!(li.value(), Node::Element(e) if e.name() == "li"))
                        .map(|li| inline_text(li).trim().to_string())
                        .filter(|t| !t.is_empty())
                        .map(|t| format!("- {t}"))
                        .collect();
                    if !items.is_empty() {
                        blocks.push(items.join("\n"));
                    }
                }
                name if INLINE_ELEMENTS.contains(&name) => inline_element_into(child, element, &mut pending),
                _ => {
                    flush_pending(&mut pending, blocks);
                    render_blocks(child, blocks);
                }
            },
            Node::Text(text) => pending.push_str(&text.text),
            _ => {}
        }
    }
    flush_pending(&mut pending, blocks);
}

fn flush_pending(pending: &mut String, blocks: &mut Vec<String>) {
    let text = pending.trim();
    if !text.is_empty() {
        blocks.push(wrap_text(text));
    }
    pending.clear();
}

fn inline_text(node: NodeRef<'_, Node>) -> String {
    let mut out = String::new();
    inline_into(node, &mut out);
    out
}

fn inline_into(node: NodeRef<'_, Node>, out: &mut String) {
    for child in node.children() {
        match child.value() {
            Node::Text(text) => out.push_str(&text.text),
            Node::Element(element) => inline_element_into(child, element, out),
            _ => {}
        }
    }
}

fn inline_element_into(child: NodeRef<'_, Node>, element: &scraper::node::Element, out: &mut String) {
    match element.name() {
        "code" => {
            let mut inner = String::new();
            inline_into(child, &mut inner);
            out.push('`');
            out.push_str(inner.trim());
            out.push('`');
        }
        "sup" => {
            out.push('^');
            inline_into(child, out);
        }
        "br" => out.push('\n'),
        "img" => {
            if let Some(src) = element.attr("src") {
                out.push_str(src);
            }
        }
        _ => inline_into(child, out),
    }
}

fn raw_text(node: NodeRef<'_, Node>) -> String {
    let mut out = String::new();
    raw_into(node, &mut out);
    out
}

fn raw_into(node: NodeRef<'_, Node>, out: &mut String) {
    for child in node.children() {
        match child.value() {
            Node::Text(text) => out.push_str(&text.text),
            Node::Element(_) => raw_into(child, out),
            _ => {}
        }
    }
}

/// Re-wrap prose at [`WRAP_WIDTH`], collapsing HTML whitespace but preserving explicit `<br>`s.
fn wrap_text(text: &str) -> String {
    text.lines()
        .map(|line| {
            let mut out = String::new();
            let mut width = 0;
            for word in line.split_whitespace() {
                if width > 0 && width + 1 + word.len() > WRAP_WIDTH {
                    out.push('\n');
                    width = 0;
                } else if width > 0 {
                    out.push(' ');
                    width += 1;
                }
                out.push_str(word);
                width += word.len();
            }
            out
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Pull `Input:` / `Output:` pairs out of the statement's example blocks.
pub fn extract_examples(html: &str) -> Vec<RawExample> {
    static PRE: LazyLock<Selector> = LazyLock::new(|| Selector::parse("pre").expect("valid selector"));
    static EXAMPLE_BLOCK: LazyLock<Selector> =
        LazyLock::new(|| Selector::parse("div.example-block").expect("valid selector"));

    let doc = Html::parse_fragment(html);
    let from = |selector: &Selector| {
        doc.select(selector)
            .filter_map(|el| parse_example_text(&el.text().collect::<String>()))
            .collect::<Vec<_>>()
    };
    let examples = from(&PRE);
    if examples.is_empty() {
        from(&EXAMPLE_BLOCK)
    } else {
        examples
    }
}

fn parse_example_text(text: &str) -> Option<RawExample> {
    static EXAMPLE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)Input:\s*(.*?)\s*Output:\s*(.*?)\s*(?:Explanation:.*)?$").expect("valid regex")
    });
    let caps = EXAMPLE.captures(text)?;
    Some(RawExample {
        input: caps[1].trim().to_string(),
        output: caps[2].trim().to_string(),
    })
}

/// Build one [`ExampleTest`] per example in the statement: a real assertion when the example
/// translates cleanly, otherwise a stub carrying the raw text.
pub fn build_example_tests(html: &str, meta: Option<&MetaData>) -> Vec<ExampleTest> {
    extract_examples(html)
        .into_iter()
        .map(|example| {
            meta.and_then(|m| translate(&example, m)).unwrap_or(ExampleTest::Stub {
                input: example.input,
                output: example.output,
            })
        })
        .collect()
}

fn translate(example: &RawExample, meta: &MetaData) -> Option<ExampleTest> {
    let ret = meta.ret.as_ref()?;
    let values = split_named_args(&example.input, meta)?;
    let args = values
        .iter()
        .zip(&meta.params)
        .map(|(value, param)| rust_literal(value, &param.ty))
        .collect::<Option<Vec<_>>>()?;
    let expected = rust_literal(&example.output, &ret.ty)?;
    Some(ExampleTest::Assert {
        args,
        expected,
        approx: ret.ty == "double",
    })
}

/// Split `nums = [2,7,11,15], target = 9` into per-parameter value strings, ordered to match
/// `meta.params` (by name when the example labels them, positionally otherwise).
fn split_named_args(input: &str, meta: &MetaData) -> Option<Vec<String>> {
    let pieces = split_top_level(input);
    if pieces.len() != meta.params.len() {
        return None;
    }
    let named: Vec<(Option<&str>, &str)> = pieces
        .iter()
        .map(|piece| match piece.split_once('=') {
            Some((name, value)) if !name.trim().is_empty() && is_identifier(name.trim()) => {
                (Some(name.trim()), value.trim())
            }
            _ => (None, piece.trim()),
        })
        .collect();
    let by_name: Vec<String> = meta
        .params
        .iter()
        .filter_map(|param| {
            named
                .iter()
                .find(|(name, _)| *name == Some(param.name.as_str()))
                .map(|(_, value)| (*value).to_string())
        })
        .collect();
    if by_name.len() == meta.params.len() {
        return Some(by_name);
    }
    // Positional fallback: strip any `name =` prefixes and take the values in order.
    Some(named.into_iter().map(|(_, value)| value.to_string()).collect())
}

fn is_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Split on commas at bracket depth zero and outside string literals.
fn split_top_level(input: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut depth = 0_i32;
    let mut in_string = false;
    for c in input.chars() {
        match c {
            '"' => {
                in_string = !in_string;
                current.push(c);
            }
            '[' if !in_string => {
                depth += 1;
                current.push(c);
            }
            ']' if !in_string => {
                depth -= 1;
                current.push(c);
            }
            ',' if !in_string && depth == 0 => {
                pieces.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        pieces.push(current.trim().to_string());
    }
    pieces
}

/// Translate a LeetCode example literal into a Rust expression for the given metaData type.
/// Returns `None` for anything beyond scalars/strings/chars and (nested) arrays of them.
fn rust_literal(value: &str, ty: &str) -> Option<String> {
    let value = value.trim();
    if let Some(inner_ty) = ty.strip_suffix("[]") {
        let inner = value.strip_prefix('[')?.strip_suffix(']')?;
        let elements = if inner.trim().is_empty() {
            Vec::new()
        } else {
            split_top_level(inner)
        };
        let literals = elements
            .iter()
            .map(|element| rust_literal(element, inner_ty))
            .collect::<Option<Vec<_>>>()?;
        return Some(format!("vec![{}]", literals.join(", ")));
    }
    match ty {
        "integer" | "long" => value.parse::<i64>().ok().map(|_| value.to_string()),
        "boolean" => matches!(value, "true" | "false").then(|| value.to_string()),
        "double" => {
            value.parse::<f64>().ok()?;
            if value.contains(['.', 'e', 'E']) {
                Some(value.to_string())
            } else {
                Some(format!("{value}.0"))
            }
        }
        "string" => (value.len() >= 2 && value.starts_with('"') && value.ends_with('"'))
            .then(|| format!("String::from({value})")),
        "character" => {
            let inner = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))?;
            let mut chars = inner.chars();
            let c = chars.next()?;
            chars.next().is_none().then(|| format!("'{c}'"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::fetch::MetaData;

    const TWO_SUM_HTML: &str = r#"<p>Given an array of integers <code>nums</code>&nbsp;and an integer <code>target</code>, return <em>indices of the two numbers such that they add up to <code>target</code></em>.</p>

<p><strong class="example">Example 1:</strong></p>

<pre>
<strong>Input:</strong> nums = [2,7,11,15], target = 9
<strong>Output:</strong> [0,1]
<strong>Explanation:</strong> Because nums[0] + nums[1] == 9, we return [0, 1].
</pre>

<p><strong class="example">Example 2:</strong></p>

<pre>
<strong>Input:</strong> nums = [3,2,4], target = 6
<strong>Output:</strong> [1,2]
</pre>

<p><strong>Constraints:</strong></p>

<ul>
	<li><code>2 &lt;= nums.length &lt;= 10<sup>4</sup></code></li>
	<li><code>-10<sup>9</sup> &lt;= nums[i] &lt;= 10<sup>9</sup></code></li>
</ul>
"#;

    fn two_sum_meta() -> MetaData {
        serde_json::from_str(
            r#"{"name":"twoSum","params":[{"name":"nums","type":"integer[]"},{"name":"target","type":"integer"}],"return":{"type":"integer[]"}}"#,
        )
        .unwrap()
    }

    #[test]
    fn doc_text_renders_code_sup_and_pre() {
        let doc = html_to_doc_text(TWO_SUM_HTML);
        assert!(doc.contains("`nums`"), "code tags become backticks: {doc}");
        assert!(doc.contains("10^4"), "sup becomes caret: {doc}");
        assert!(doc.contains("```text\nInput: nums = [2,7,11,15], target = 9"), "{doc}");
        assert!(doc.contains("- `2 <= nums.length <= 10^4`"), "{doc}");
        assert!(
            !doc.contains("<p>") && !doc.contains("</") && !doc.contains("<strong>"),
            "no tags survive: {doc}"
        );
    }

    #[test]
    fn loose_inline_content_flows_into_one_paragraph() {
        // Two Sum's follow-up is bare text with inline tags, not wrapped in <p>.
        let html = "<strong>Follow-up:</strong> Can you do better than <code>O(n<sup>2</sup>)</code> time complexity?";
        assert_eq!(
            html_to_doc_text(html),
            "Follow-up: Can you do better than `O(n^2)` time complexity?"
        );
    }

    #[test]
    fn wraps_long_paragraphs() {
        let long = format!("<p>{}</p>", "word ".repeat(60));
        for line in html_to_doc_text(&long).lines() {
            assert!(line.len() <= WRAP_WIDTH, "line too long: {line}");
        }
    }

    #[test]
    fn extracts_examples() {
        let examples = extract_examples(TWO_SUM_HTML);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].input, "nums = [2,7,11,15], target = 9");
        assert_eq!(examples[0].output, "[0,1]");
        assert_eq!(examples[1].output, "[1,2]");
    }

    #[test]
    fn extracts_example_block_divs() {
        let html =
            r#"<div class="example-block"><p><strong>Input:</strong> n = 3</p><p><strong>Output:</strong> 5</p></div>"#;
        let examples = extract_examples(html);
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].input, "n = 3");
        assert_eq!(examples[0].output, "5");
    }

    #[test]
    fn translates_two_sum() {
        let tests = build_example_tests(TWO_SUM_HTML, Some(&two_sum_meta()));
        assert_eq!(
            tests[0],
            ExampleTest::Assert {
                args: vec!["vec![2, 7, 11, 15]".into(), "9".into()],
                expected: "vec![0, 1]".into(),
                approx: false,
            }
        );
    }

    #[test]
    fn split_top_level_respects_brackets_and_strings() {
        assert_eq!(
            split_top_level(r#"s = "a,b", nums = [[1,2],[3,4]], k = 2"#),
            vec![r#"s = "a,b""#, "nums = [[1,2],[3,4]]", "k = 2"]
        );
    }

    #[test]
    fn literal_translation() {
        assert_eq!(rust_literal("9", "integer").unwrap(), "9");
        assert_eq!(rust_literal("[2,7]", "integer[]").unwrap(), "vec![2, 7]");
        assert_eq!(
            rust_literal("[[1,2],[3]]", "integer[][]").unwrap(),
            "vec![vec![1, 2], vec![3]]"
        );
        assert_eq!(rust_literal("[]", "integer[]").unwrap(), "vec![]");
        assert_eq!(rust_literal(r#""abc""#, "string").unwrap(), r#"String::from("abc")"#);
        assert_eq!(
            rust_literal(r#"["a","b"]"#, "string[]").unwrap(),
            r#"vec![String::from("a"), String::from("b")]"#
        );
        assert_eq!(rust_literal(r#""x""#, "character").unwrap(), "'x'");
        assert_eq!(rust_literal("true", "boolean").unwrap(), "true");
        assert_eq!(rust_literal("2", "double").unwrap(), "2.0");
        assert_eq!(rust_literal("2.5", "double").unwrap(), "2.5");
        assert_eq!(rust_literal("[1,2]", "ListNode"), None);
        assert_eq!(rust_literal("null", "integer"), None);
    }

    #[test]
    fn unparsable_examples_become_stubs() {
        let html = r"<pre>
<strong>Input:</strong> l1 = [2,4,3], l2 = [5,6,4]
<strong>Output:</strong> [7,0,8]
</pre>";
        let meta: MetaData = serde_json::from_str(
            r#"{"name":"addTwoNumbers","params":[{"name":"l1","type":"ListNode"},{"name":"l2","type":"ListNode"}],"return":{"type":"ListNode"}}"#,
        )
        .unwrap();
        let tests = build_example_tests(html, Some(&meta));
        assert_eq!(
            tests[0],
            ExampleTest::Stub {
                input: "l1 = [2,4,3], l2 = [5,6,4]".into(),
                output: "[7,0,8]".into(),
            }
        );
    }

    #[test]
    fn no_meta_means_stubs() {
        let tests = build_example_tests(TWO_SUM_HTML, None);
        assert_eq!(tests.len(), 2);
        assert!(matches!(tests[0], ExampleTest::Stub { .. }));
    }

    #[test]
    fn arg_count_mismatch_is_a_stub() {
        let html = "<pre>Input: nums = [1,2], target = 3, extra = 4\nOutput: [0,1]</pre>";
        let tests = build_example_tests(html, Some(&two_sum_meta()));
        assert!(matches!(tests[0], ExampleTest::Stub { .. }));
    }
}
