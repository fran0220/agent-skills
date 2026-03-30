use once_cell::sync::Lazy;
use reqwest::Url;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::providers::SearchResult;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchIntent {
    Factual,
    Status,
    Comparison,
    Tutorial,
    Exploratory,
    News,
    Resource,
}

impl SearchIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Factual => "factual",
            Self::Status => "status",
            Self::Comparison => "comparison",
            Self::Tutorial => "tutorial",
            Self::Exploratory => "exploratory",
            Self::News => "news",
            Self::Resource => "resource",
        }
    }
}

struct IntentWeights {
    keyword: f64,
    freshness: f64,
    authority: f64,
}

const DEFAULT_AUTHORITY_SCORE: f64 = 0.4;

static AUTHORITY_DOMAINS: Lazy<HashMap<&'static str, f64>> = Lazy::new(|| {
    HashMap::from([
        ("github.com", 1.0),
        ("gitlab.com", 1.0),
        ("stackoverflow.com", 1.0),
        ("wikipedia.org", 1.0),
        ("arxiv.org", 1.0),
        ("docs.python.org", 1.0),
        ("docs.rs", 1.0),
        ("doc.rust-lang.org", 1.0),
        ("developer.mozilla.org", 1.0),
        ("developer.apple.com", 1.0),
        ("developer.android.com", 1.0),
        ("cloud.google.com", 1.0),
        ("docs.aws.amazon.com", 1.0),
        ("learn.microsoft.com", 1.0),
        ("react.dev", 1.0),
        ("vuejs.org", 1.0),
        ("nodejs.org", 1.0),
        ("go.dev", 1.0),
        ("kotlinlang.org", 1.0),
        ("typescriptlang.org", 1.0),
        ("swift.org", 1.0),
        ("docs.docker.com", 1.0),
        ("kubernetes.io", 1.0),
        ("rust-lang.org", 1.0),
        ("tc39.es", 1.0),
        ("w3.org", 1.0),
        ("datatracker.ietf.org", 1.0),
        ("peps.python.org", 1.0),
        ("crates.io", 1.0),
        ("pypi.org", 1.0),
        ("npmjs.com", 1.0),
        ("pkg.go.dev", 1.0),
        ("news.ycombinator.com", 0.8),
        ("lobste.rs", 0.8),
        ("stackexchange.com", 0.8),
        ("serverfault.com", 0.8),
        ("superuser.com", 0.8),
        ("askubuntu.com", 0.8),
        ("reddit.com", 0.8),
        ("dev.to", 0.8),
        ("css-tricks.com", 0.8),
        ("smashingmagazine.com", 0.8),
        ("web.dev", 0.8),
        ("blog.cloudflare.com", 0.8),
        ("engineering.fb.com", 0.8),
        ("netflixtechblog.com", 0.8),
        ("openai.com", 0.8),
        ("anthropic.com", 0.8),
        ("huggingface.co", 0.8),
        ("papers.nips.cc", 0.8),
        ("aclanthology.org", 0.8),
        ("distill.pub", 0.8),
        ("medium.com", 0.6),
        ("towardsdatascience.com", 0.6),
        ("freecodecamp.org", 0.6),
        ("baeldung.com", 0.6),
        ("digitalocean.com", 0.6),
        ("tutorialspoint.com", 0.6),
        ("geeksforgeeks.org", 0.6),
        ("realpython.com", 0.6),
        ("hackernoon.com", 0.6),
        ("infoq.com", 0.6),
        ("thenewstack.io", 0.6),
        ("techcrunch.com", 0.6),
        ("arstechnica.com", 0.6),
        ("theverge.com", 0.6),
        ("wired.com", 0.6),
        ("36kr.com", 0.6),
        ("sspai.com", 0.6),
        ("juejin.cn", 0.6),
        ("segmentfault.com", 0.6),
        ("cnblogs.com", 0.6),
        ("zhihu.com", 0.6),
    ])
});

pub fn detect_intent(query: &str) -> SearchIntent {
    let lower = query.to_ascii_lowercase();

    if contains_any(
        &lower,
        &[
            "latest",
            "recent",
            "today",
            "current",
            "breaking",
            "news",
            "announcement",
            "released",
            "最新",
            "近期",
            "最近",
            "当前",
            "今天",
            "新闻",
        ],
    ) {
        return if contains_any(
            &lower,
            &[
                "status", "uptime", "incident", "outage", "down", "health", "状态", "故障", "宕机",
            ],
        ) {
            SearchIntent::Status
        } else {
            SearchIntent::News
        };
    }

    if contains_any(
        &lower,
        &[
            "vs",
            "versus",
            "compare",
            "comparison",
            "区别",
            "对比",
            "比较",
        ],
    ) {
        return SearchIntent::Comparison;
    }

    if contains_any(
        &lower,
        &[
            "how to",
            "guide",
            "tutorial",
            "example",
            "step by step",
            "怎么",
            "教程",
            "指南",
            "示例",
        ],
    ) {
        return SearchIntent::Tutorial;
    }

    if contains_any(
        &lower,
        &[
            "docs",
            "documentation",
            "reference",
            "repository",
            "github",
            "api",
            "文档",
            "仓库",
            "参考",
        ],
    ) {
        return SearchIntent::Resource;
    }

    if contains_any(
        &lower,
        &[
            "what is",
            "who is",
            "when did",
            "where is",
            "定义",
            "是什么",
            "谁是",
            "何时",
        ],
    ) {
        return SearchIntent::Factual;
    }

    SearchIntent::Exploratory
}

pub fn dedup_results(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen = HashMap::<String, SearchResult>::new();
    let mut order = Vec::new();

    for mut result in results {
        let key = normalize_url(&result.url);
        if let Some(existing) = seen.get_mut(&key) {
            if !existing
                .source
                .split(',')
                .any(|src| src.trim() == result.source)
            {
                existing.source = format!("{},{}", existing.source, result.source);
            }
            if result.snippet.len() > existing.snippet.len() {
                existing.snippet = std::mem::take(&mut result.snippet);
            }
            if result.title.len() > existing.title.len() {
                existing.title = std::mem::take(&mut result.title);
            }
            if existing.published_date.is_none() {
                existing.published_date = result.published_date.take();
            }
            continue;
        }

        result.url = key.clone();
        seen.insert(key.clone(), result);
        order.push(key);
    }

    order
        .into_iter()
        .filter_map(|key| seen.remove(&key))
        .collect()
}

pub fn score_results(results: &mut [SearchResult], query: &str, intent: SearchIntent) {
    for result in results.iter_mut() {
        result.score = score_result(result, query, intent);
    }
    results.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

pub fn normalize_url(url: &str) -> String {
    let Ok(mut parsed) = Url::parse(url) else {
        return url.trim_end_matches('/').to_string();
    };

    parsed.set_fragment(None);

    let mut pairs = parsed
        .query_pairs()
        .filter(|(key, _)| !key.starts_with("utm_"))
        .collect::<Vec<_>>();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));

    if pairs.is_empty() {
        parsed.set_query(None);
    } else {
        let query = pairs
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("&");
        parsed.set_query(Some(&query));
    }

    let normalized_path = parsed.path().trim_end_matches('/').to_string();
    parsed.set_path(if normalized_path.is_empty() {
        "/"
    } else {
        &normalized_path
    });
    parsed.to_string().trim_end_matches('/').to_string()
}

fn score_result(result: &SearchResult, query: &str, intent: SearchIntent) -> f64 {
    let weights = weights_for(intent);
    let keyword = keyword_score(result, query);
    let freshness = freshness_score(result);
    let authority = authority_score(&result.url);
    round_score(
        weights.keyword * keyword + weights.freshness * freshness + weights.authority * authority,
    )
}

fn keyword_score(result: &SearchResult, query: &str) -> f64 {
    let terms = query_terms(query);
    if terms.is_empty() {
        return 0.5;
    }

    let haystack = format!("{} {}", result.title, result.snippet).to_ascii_lowercase();
    let matches = terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count();
    matches as f64 / terms.len() as f64
}

fn freshness_score(result: &SearchResult) -> f64 {
    use chrono::{DateTime, NaiveDate, Utc};

    let Some(date_str) = result.published_date.as_deref() else {
        return 0.5;
    };

    if let Ok(parsed) = DateTime::parse_from_rfc3339(date_str) {
        return freshness_from_days((Utc::now() - parsed.with_timezone(&Utc)).num_days());
    }

    if let Ok(parsed) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let parsed = parsed.and_hms_opt(0, 0, 0).unwrap().and_utc();
        return freshness_from_days((Utc::now() - parsed).num_days());
    }

    0.5
}

fn freshness_from_days(days_old: i64) -> f64 {
    match days_old {
        i64::MIN..=1 => 1.0,
        2..=7 => 0.9,
        8..=30 => 0.7,
        31..=90 => 0.5,
        91..=365 => 0.3,
        _ => 0.1,
    }
}

fn authority_score(url: &str) -> f64 {
    let Ok(parsed) = Url::parse(url) else {
        return DEFAULT_AUTHORITY_SCORE;
    };
    let host = parsed
        .host_str()
        .unwrap_or_default()
        .trim_start_matches("www.");

    if let Some(score) = AUTHORITY_DOMAINS.get(host) {
        return *score;
    }

    for (domain, score) in AUTHORITY_DOMAINS.iter() {
        if host.ends_with(&format!(".{domain}")) {
            return *score;
        }
    }

    if host.starts_with("docs.") {
        return 0.9;
    }
    if host.ends_with(".github.io") {
        return 0.7;
    }
    if host.starts_with("blog.") {
        return 0.6;
    }
    if host.ends_with(".edu") || host.ends_with(".gov") {
        return 0.8;
    }

    DEFAULT_AUTHORITY_SCORE
}

fn query_terms(query: &str) -> HashSet<String> {
    query
        .split(|ch: char| !ch.is_alphanumeric() && ch != '-' && ch != '_')
        .filter(|term| term.chars().count() > 2)
        .map(|term| term.to_ascii_lowercase())
        .collect()
}

fn contains_any(query: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| query.contains(needle))
}

fn weights_for(intent: SearchIntent) -> IntentWeights {
    match intent {
        SearchIntent::Factual => IntentWeights {
            keyword: 0.4,
            freshness: 0.1,
            authority: 0.5,
        },
        SearchIntent::Status => IntentWeights {
            keyword: 0.3,
            freshness: 0.5,
            authority: 0.2,
        },
        SearchIntent::Comparison => IntentWeights {
            keyword: 0.4,
            freshness: 0.2,
            authority: 0.4,
        },
        SearchIntent::Tutorial => IntentWeights {
            keyword: 0.4,
            freshness: 0.1,
            authority: 0.5,
        },
        SearchIntent::Exploratory => IntentWeights {
            keyword: 0.3,
            freshness: 0.2,
            authority: 0.5,
        },
        SearchIntent::News => IntentWeights {
            keyword: 0.3,
            freshness: 0.6,
            authority: 0.1,
        },
        SearchIntent::Resource => IntentWeights {
            keyword: 0.5,
            freshness: 0.1,
            authority: 0.4,
        },
    }
}

fn round_score(score: f64) -> f64 {
    (score * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_strips_tracking_and_fragment() {
        let normalized = normalize_url("https://example.com/path/?utm_source=test&b=2&a=1#section");
        assert_eq!(normalized, "https://example.com/path?a=1&b=2");
    }
}
