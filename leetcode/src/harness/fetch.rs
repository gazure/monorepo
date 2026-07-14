//! LeetCode GraphQL client: problem-index lookup (cached on disk) and per-problem fetching.

use std::{fs, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const GRAPHQL_URL: &str = "https://leetcode.com/graphql";
const REFERER: &str = "https://leetcode.com/problemset/";
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

const INDEX_QUERY: &str = r"
query problemsetQuestionList($categorySlug: String, $limit: Int, $skip: Int, $filters: QuestionListFilterInput) {
  questionList(categorySlug: $categorySlug, limit: $limit, skip: $skip, filters: $filters) {
    data {
      questionFrontendId
      title
      titleSlug
      difficulty
      isPaidOnly
    }
  }
}";

const QUESTION_QUERY: &str = r"
query questionData($titleSlug: String!) {
  question(titleSlug: $titleSlug) {
    questionFrontendId
    title
    titleSlug
    difficulty
    isPaidOnly
    content
    codeSnippets {
      lang
      langSlug
      code
    }
    exampleTestcaseList
    metaData
  }
}";

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("leetcode api error: {0}")]
    Api(String),
    #[error("problem {0} ({1}) is paid-only; only free problems can be fetched")]
    PaidOnly(String, String),
    #[error("no problem with id {0} in the index (try --refresh-index)")]
    UnknownId(u32),
    #[error("problem has no rust code snippet")]
    NoRustSnippet,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// One row of the cached id → slug index.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEntry {
    pub question_frontend_id: String,
    pub title: String,
    pub title_slug: String,
    pub difficulty: String,
    pub is_paid_only: bool,
}

/// Full problem data from the `questionData` query.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub question_frontend_id: String,
    pub title: String,
    pub title_slug: String,
    pub difficulty: String,
    pub is_paid_only: bool,
    pub content: Option<String>,
    #[serde(default)]
    pub code_snippets: Vec<CodeSnippet>,
    #[serde(default)]
    pub example_testcase_list: Vec<String>,
    #[serde(default)]
    pub meta_data: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeSnippet {
    pub lang: String,
    pub lang_slug: String,
    pub code: String,
}

impl Question {
    pub fn rust_snippet(&self) -> Result<&str, FetchError> {
        self.code_snippets
            .iter()
            .find(|s| s.lang_slug == "rust")
            .map(|s| s.code.as_str())
            .ok_or(FetchError::NoRustSnippet)
    }

    /// Parsed `metaData`, if it has the plain function shape (design problems use a different
    /// shape and yield `None`, which downstream code treats as "can't auto-translate examples").
    pub fn meta(&self) -> Option<MetaData> {
        serde_json::from_str(&self.meta_data).ok()
    }
}

/// The function-shaped subset of LeetCode's `metaData` JSON string.
#[derive(Debug, Deserialize)]
pub struct MetaData {
    pub name: String,
    #[serde(default)]
    pub params: Vec<Param>,
    #[serde(rename = "return")]
    pub ret: Option<RetType>,
}

#[derive(Debug, Deserialize)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Deserialize)]
pub struct RetType {
    #[serde(rename = "type")]
    pub ty: String,
}

pub struct Client {
    http: reqwest::Client,
}

impl Client {
    pub fn new() -> Result<Self, FetchError> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { http })
    }

    async fn graphql(&self, query: &str, variables: serde_json::Value) -> Result<serde_json::Value, FetchError> {
        let body = serde_json::json!({ "query": query, "variables": variables });
        let response = self
            .http
            .post(GRAPHQL_URL)
            .header("Referer", REFERER)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let value: serde_json::Value = response.json().await?;
        if let Some(errors) = value.get("errors") {
            return Err(FetchError::Api(errors.to_string()));
        }
        Ok(value)
    }

    /// Fetch one page of the problem catalog. The API caps page size at [`INDEX_PAGE_SIZE`].
    pub async fn fetch_index_page(&self, skip: usize) -> Result<Vec<IndexEntry>, FetchError> {
        tracing::info!(skip, "downloading problem index page from leetcode");
        let variables =
            serde_json::json!({ "categorySlug": "", "skip": skip, "limit": INDEX_PAGE_SIZE, "filters": {} });
        let value = self.graphql(INDEX_QUERY, variables).await?;
        let data = value
            .pointer("/data/questionList/data")
            .cloned()
            .ok_or_else(|| FetchError::Api("missing questionList.data in response".into()))?;
        Ok(serde_json::from_value(data)?)
    }

    pub async fn fetch_question(&self, slug: &str) -> Result<Question, FetchError> {
        tracing::info!(slug, "fetching problem");
        let variables = serde_json::json!({ "titleSlug": slug });
        let value = self.graphql(QUESTION_QUERY, variables).await?;
        let question = value
            .pointer("/data/question")
            .filter(|q| !q.is_null())
            .cloned()
            .ok_or_else(|| FetchError::Api(format!("no question data returned for slug '{slug}'")))?;
        Ok(serde_json::from_value(question)?)
    }
}

/// Where the id → slug index cache lives, following the repo's `data/<crate>` convention.
pub fn index_cache_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/leetcode/problem_index.json")
}

/// The API silently caps `limit` at 100, so the index is fetched page by page.
const INDEX_PAGE_SIZE: usize = 100;
/// Safety bound on how much catalog we'll page through hunting for a nonexistent id.
const MAX_INDEX: usize = 10_000;

/// Load the cached index, extending it page by page until it contains `id` (or the catalog runs
/// out). `refresh` discards the cache first.
pub async fn load_or_fetch_index(client: &Client, id: u32, refresh: bool) -> Result<Vec<IndexEntry>, FetchError> {
    let path = index_cache_path();
    let mut index: Vec<IndexEntry> = if refresh || !path.exists() {
        Vec::new()
    } else {
        serde_json::from_str(&fs::read_to_string(&path)?)?
    };

    let id_str = id.to_string();
    let mut extended = false;
    while !index.iter().any(|entry| entry.question_frontend_id == id_str) && index.len() < MAX_INDEX {
        let page = client.fetch_index_page(index.len()).await?;
        let exhausted = page.len() < INDEX_PAGE_SIZE;
        index.extend(page);
        extended = true;
        if exhausted {
            break;
        }
    }

    if extended {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, serde_json::to_string_pretty(&index)?)?;
        tracing::info!(path = %path.display(), problems = index.len(), "wrote problem index cache");
    }
    Ok(index)
}

/// Find a problem by frontend id. Note the API models ids as strings.
pub fn lookup(index: &[IndexEntry], id: u32) -> Result<&IndexEntry, FetchError> {
    let id_str = id.to_string();
    index
        .iter()
        .find(|entry| entry.question_frontend_id == id_str)
        .ok_or(FetchError::UnknownId(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_function_meta_data() {
        let raw = r#"{"name":"twoSum","params":[{"name":"nums","type":"integer[]"},{"name":"target","type":"integer"}],"return":{"type":"integer[]"}}"#;
        let meta: MetaData = serde_json::from_str(raw).unwrap();
        assert_eq!(meta.name, "twoSum");
        assert_eq!(meta.params.len(), 2);
        assert_eq!(meta.params[0].ty, "integer[]");
        assert_eq!(meta.ret.unwrap().ty, "integer[]");
    }

    #[test]
    fn design_meta_data_yields_none() {
        let question = Question {
            question_frontend_id: "146".into(),
            title: "LRU Cache".into(),
            title_slug: "lru-cache".into(),
            difficulty: "Medium".into(),
            is_paid_only: false,
            content: None,
            code_snippets: vec![],
            example_testcase_list: vec![],
            meta_data: r#"{"classname":"LRUCache","constructor":{"params":[]}}"#.into(),
        };
        assert!(question.meta().is_none());
    }

    #[test]
    fn lookup_matches_string_ids() {
        let index = vec![IndexEntry {
            question_frontend_id: "1".into(),
            title: "Two Sum".into(),
            title_slug: "two-sum".into(),
            difficulty: "Easy".into(),
            is_paid_only: false,
        }];
        assert_eq!(lookup(&index, 1).unwrap().title_slug, "two-sum");
        assert!(matches!(lookup(&index, 2), Err(FetchError::UnknownId(2))));
    }
}
