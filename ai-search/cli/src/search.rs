use anyhow::{anyhow, Result};
use futures::future::join_all;
use serde::Serialize;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::client::{AIClient, Message};
use crate::config::AppConfig;
use crate::providers::exa::ExaSearchProvider;
use crate::providers::grok::GrokSearchProvider;
use crate::providers::tavily::TavilySearchProvider;
use crate::providers::{SearchProvider, SearchResult as ProviderSearchResult};
use crate::scoring::{dedup_results, detect_intent, score_results, SearchIntent};

const DEFAULT_RESULTS_PER_PROVIDER: u32 = 5;
const MAX_PROVIDER_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    Fast,
    Deep,
    Answer,
}

impl SearchMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Deep => "deep",
            Self::Answer => "answer",
        }
    }
}

impl fmt::Display for SearchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SearchMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fast" => Ok(Self::Fast),
            "deep" => Ok(Self::Deep),
            "answer" => Ok(Self::Answer),
            other => Err(anyhow!("invalid search mode: {other}")),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub mode: SearchMode,
    pub model: String,
    pub content: String,
    pub providers: Vec<String>,
    pub results: Vec<ProviderSearchResult>,
    pub tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_results: Option<Vec<SubQueryResult>>,
}

#[derive(Debug, Serialize)]
pub struct SubQueryResult {
    pub sub_query: String,
    pub content: String,
    pub providers: Vec<String>,
    pub results: Vec<ProviderSearchResult>,
    pub tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
}

#[derive(Clone)]
pub struct SearchEngine {
    ai_client: AIClient,
    grok: Option<Arc<GrokSearchProvider>>,
    exa: Option<Arc<dyn SearchProvider>>,
    tavily: Option<Arc<dyn SearchProvider>>,
    tavily_answer: Option<Arc<TavilySearchProvider>>,
    config: AppConfig,
    semaphore: Arc<Semaphore>,
}

struct SearchOutcome {
    content: String,
    results: Vec<ProviderSearchResult>,
    providers: Vec<String>,
    answer: Option<String>,
    tokens: u32,
}

impl SearchEngine {
    pub fn new(config: AppConfig) -> Result<Self> {
        let grok = if config.api_key.is_empty() {
            None
        } else {
            Some(Arc::new(GrokSearchProvider::new(
                config.api_url.clone(),
                config.api_key.clone(),
                config.search_model.clone(),
                config.timeout_secs,
            )?))
        };

        let exa = if config.exa_key.is_empty() {
            None
        } else {
            Some(Arc::new(ExaSearchProvider::new(
                config.exa_key.clone(),
                config.timeout_secs,
            )?) as Arc<dyn SearchProvider>)
        };

        let tavily_answer = if config.tavily_key.is_empty() {
            None
        } else {
            Some(Arc::new(TavilySearchProvider::new(
                config.tavily_key.clone(),
                config.timeout_secs,
            )?))
        };
        let tavily = tavily_answer
            .as_ref()
            .map(|provider| provider.clone() as Arc<dyn SearchProvider>);

        Ok(Self {
            ai_client: AIClient::new(config.clone()),
            grok,
            exa,
            tavily,
            tavily_answer,
            config,
            semaphore: Arc::new(Semaphore::new(MAX_PROVIDER_CONCURRENCY)),
        })
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn has_llm_proxy(&self) -> bool {
        !self.config.api_key.trim().is_empty()
    }

    pub async fn search(
        &self,
        query: &str,
        mode: SearchMode,
        model: Option<&str>,
        split: u32,
    ) -> Result<SearchResponse> {
        let selected_model = self.response_model(mode, model);
        let split = split.max(1);
        let intent = detect_intent(query);
        let sub_queries = self.split_query(query, split).await?;

        if sub_queries.len() <= 1 {
            let outcome = self.search_single(query, mode, model, intent).await?;
            return Ok(SearchResponse {
                query: query.to_string(),
                mode,
                model: selected_model,
                content: outcome.content,
                providers: outcome.providers,
                results: outcome.results,
                tokens: outcome.tokens,
                answer: outcome.answer,
                intent: Some(intent.as_str()),
                sub_results: None,
            });
        }

        let tasks = sub_queries.into_iter().map(|sub_query| {
            let engine = self.clone();
            let requested_model = model.map(ToOwned::to_owned);
            tokio::spawn(async move {
                let intent = detect_intent(&sub_query);
                let outcome = engine
                    .search_single(&sub_query, mode, requested_model.as_deref(), intent)
                    .await?;
                Ok::<SubQueryResult, anyhow::Error>(SubQueryResult {
                    sub_query,
                    content: outcome.content,
                    providers: outcome.providers,
                    results: outcome.results,
                    tokens: outcome.tokens,
                    answer: outcome.answer,
                })
            })
        });

        let mut merged_results = Vec::new();
        let mut merged_providers = Vec::new();
        let mut merged_answer = None;
        let mut total_tokens = 0;
        let mut sub_results = Vec::new();

        for task in join_all(tasks).await {
            let sub_result = task??;
            total_tokens += sub_result.tokens;
            merged_results.extend(sub_result.results.clone());
            extend_unique(&mut merged_providers, &sub_result.providers);
            if merged_answer.is_none() {
                merged_answer = sub_result.answer.clone();
            }
            sub_results.push(sub_result);
        }

        let mut deduped = dedup_results(merged_results);
        score_results(&mut deduped, query, intent);

        let (content, synthesis_tokens) = self
            .merge_sub_query_results(query, &sub_results, &deduped)
            .await?;
        total_tokens += synthesis_tokens;

        Ok(SearchResponse {
            query: query.to_string(),
            mode,
            model: selected_model,
            content,
            providers: merged_providers,
            results: deduped,
            tokens: total_tokens,
            answer: merged_answer,
            intent: Some(intent.as_str()),
            sub_results: Some(sub_results),
        })
    }

    async fn search_single(
        &self,
        query: &str,
        mode: SearchMode,
        model: Option<&str>,
        intent: SearchIntent,
    ) -> Result<SearchOutcome> {
        if matches!(mode, SearchMode::Fast) {
            return self.search_fast(query, model).await;
        }

        let raw = match mode {
            SearchMode::Fast => unreachable!(),
            SearchMode::Deep => self.search_deep(query, model).await?,
            SearchMode::Answer => self.search_answer(query).await?,
        };

        let mut results = dedup_results(raw.results);
        score_results(&mut results, query, intent);

        let (content, synthesis_tokens) =
            if matches!(mode, SearchMode::Answer) && raw.answer.is_some() {
                (raw.answer.clone().unwrap(), 0)
            } else {
                self.synthesize_results(query, &results, raw.answer.as_deref())
                    .await?
            };

        Ok(SearchOutcome {
            content,
            results,
            providers: raw.providers,
            answer: raw.answer,
            tokens: raw.tokens + synthesis_tokens,
        })
    }

    async fn search_fast(&self, query: &str, model: Option<&str>) -> Result<SearchOutcome> {
        let provider = self.grok_provider(model)?;
        let Some(provider) = provider else {
            anyhow::bail!(
                "Fast mode requires [proxy].key or AI_SEARCH_KEY so Grok can run web search"
            );
        };

        let _permit = self.semaphore.clone().acquire_owned().await?;
        let content = provider
            .search_text(query, DEFAULT_RESULTS_PER_PROVIDER)
            .await?;

        Ok(SearchOutcome {
            content,
            results: Vec::new(),
            providers: vec!["grok".to_string()],
            answer: None,
            tokens: 0,
        })
    }

    async fn search_deep(&self, query: &str, model: Option<&str>) -> Result<ProviderRun> {
        let grok: Option<Arc<dyn SearchProvider>> = self
            .grok_provider(model)?
            .map(|p| p as Arc<dyn SearchProvider>);
        let exa = self.exa.clone();
        let tavily = self.tavily.clone();

        if grok.is_none() && exa.is_none() && tavily.is_none() {
            anyhow::bail!(
                "Deep mode needs at least one configured provider in [proxy], [exa], or [tavily]"
            );
        }

        let grok_future = self.run_optional_provider("grok", grok, query);
        let exa_future = self.run_optional_provider("exa", exa, query);
        let tavily_future = self.run_optional_provider("tavily", tavily, query);

        let (grok_result, exa_result, tavily_result) =
            tokio::join!(grok_future, exa_future, tavily_future);

        let mut results = Vec::new();
        let mut providers = Vec::new();
        let mut had_success = false;
        let mut errors = Vec::new();

        for outcome in [grok_result, exa_result, tavily_result] {
            match outcome {
                Ok(Some(provider_result)) => {
                    had_success = true;
                    results.extend(provider_result.results);
                    extend_unique(&mut providers, &provider_result.providers);
                }
                Ok(None) => {}
                Err(error) => errors.push(error.to_string()),
            }
        }

        if !had_success {
            anyhow::bail!(
                "All configured providers failed in deep mode: {}",
                errors.join("; ")
            );
        }

        Ok(ProviderRun {
            results,
            providers,
            answer: None,
            tokens: 0,
        })
    }

    async fn search_answer(&self, query: &str) -> Result<ProviderRun> {
        let Some(provider) = self.tavily_answer.clone() else {
            anyhow::bail!("Answer mode requires [tavily].key or TAVILY_KEY");
        };

        let _permit = self.semaphore.clone().acquire_owned().await?;
        let (results, answer) = provider
            .search_with_answer(query, DEFAULT_RESULTS_PER_PROVIDER)
            .await?;

        if results.is_empty() && answer.is_none() {
            anyhow::bail!("Tavily returned no results for answer mode");
        }

        Ok(ProviderRun {
            results,
            providers: vec!["tavily".to_string()],
            answer,
            tokens: 0,
        })
    }

    async fn run_optional_provider(
        &self,
        name: &str,
        provider: Option<Arc<dyn SearchProvider>>,
        query: &str,
    ) -> Result<Option<ProviderRun>> {
        let Some(provider) = provider else {
            return Ok(None);
        };

        match self.run_provider(provider, query).await {
            Ok(results) => Ok(Some(ProviderRun {
                results,
                providers: vec![name.to_string()],
                answer: None,
                tokens: 0,
            })),
            Err(error) => {
                tracing::warn!(provider = name, error = %error, "search provider failed");
                Err(error)
            }
        }
    }

    async fn run_provider(
        &self,
        provider: Arc<dyn SearchProvider>,
        query: &str,
    ) -> Result<Vec<ProviderSearchResult>> {
        let _permit = self.semaphore.clone().acquire_owned().await?;
        provider.search(query, DEFAULT_RESULTS_PER_PROVIDER).await
    }

    async fn split_query(&self, query: &str, max_split: u32) -> Result<Vec<String>> {
        if max_split <= 1 || !self.has_llm_proxy() {
            return Ok(vec![query.to_string()]);
        }

        let prompt = format!(
            "Split the following search query into {max_split} or fewer independent sub-questions that can be searched in parallel. Return ONLY a JSON array of strings and nothing else.\n\nQuery: {query}"
        );
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You break complex research questions into independent search queries. Return a valid JSON array of strings only."
                    .to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let (content, _) = self
            .ai_client
            .chat_raw(messages, &self.config.analysis_model)
            .await?;
        let trimmed = content.trim();
        let json_text = if trimmed.starts_with("```") {
            trimmed
                .lines()
                .skip(1)
                .take_while(|line| !line.starts_with("```"))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            trimmed.to_string()
        };

        let queries = match serde_json::from_str::<Vec<String>>(&json_text) {
            Ok(queries) => queries,
            Err(error) => {
                tracing::warn!(error = %error, "failed to parse split query response");
                return Ok(vec![query.to_string()]);
            }
        };

        let queries = queries
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .take(max_split as usize)
            .collect::<Vec<_>>();

        if queries.is_empty() {
            Ok(vec![query.to_string()])
        } else {
            Ok(queries)
        }
    }

    async fn synthesize_results(
        &self,
        query: &str,
        results: &[ProviderSearchResult],
        answer: Option<&str>,
    ) -> Result<(String, u32)> {
        if results.is_empty() {
            return Ok((answer.unwrap_or("No results found.").to_string(), 0));
        }

        if !self.has_llm_proxy() {
            return Ok((format_results(results, answer), 0));
        }

        let payload = serde_json::to_string_pretty(results)?;
        let answer_section = answer
            .map(|value| format!("Existing provider answer:\n{value}\n\n"))
            .unwrap_or_default();
        let prompt = format!(
            "Original question: {query}\n\n{answer_section}Search results JSON:\n{payload}\n\nWrite a concise but comprehensive answer that cites source URLs inline or in a numbered list at the end. Match the language of the original query."
        );
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are a research synthesis assistant. Merge search results into one coherent answer, deduplicate overlapping points, and preserve the most authoritative sources."
                    .to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let (content, usage) = self
            .ai_client
            .chat_raw(messages, &self.config.analysis_model)
            .await?;
        Ok((
            content,
            usage.as_ref().map(|item| item.total_tokens).unwrap_or(0),
        ))
    }

    async fn merge_sub_query_results(
        &self,
        original_query: &str,
        sub_results: &[SubQueryResult],
        merged_results: &[ProviderSearchResult],
    ) -> Result<(String, u32)> {
        if !self.has_llm_proxy() {
            return Ok((format_sub_queries(sub_results, merged_results), 0));
        }

        let sub_query_context = sub_results
            .iter()
            .map(|item| format!("Sub-query: {}\n{}", item.sub_query, item.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        let merged_json = serde_json::to_string_pretty(merged_results)?;
        let prompt = format!(
            "Original question: {original_query}\n\nSub-query answers:\n{sub_query_context}\n\nMerged search results JSON:\n{merged_json}\n\nProduce one final answer that synthesizes the sub-query findings, removes duplication, and includes citations."
        );
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You merge multiple research passes into a single final answer. Preserve sources, resolve overlap, and keep the response readable."
                    .to_string(),
            },
            Message {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let (content, usage) = self
            .ai_client
            .chat_raw(messages, &self.config.analysis_model)
            .await?;
        Ok((
            content,
            usage.as_ref().map(|item| item.total_tokens).unwrap_or(0),
        ))
    }

    fn response_model(&self, mode: SearchMode, model: Option<&str>) -> String {
        match mode {
            SearchMode::Answer => "tavily-answer".to_string(),
            _ => model.unwrap_or(&self.config.search_model).to_string(),
        }
    }

    fn grok_provider(&self, model: Option<&str>) -> Result<Option<Arc<GrokSearchProvider>>> {
        if self.config.api_key.is_empty() {
            return Ok(None);
        }

        if let Some(requested_model) = model {
            if requested_model != self.config.search_model {
                return Ok(Some(Arc::new(GrokSearchProvider::new(
                    self.config.api_url.clone(),
                    self.config.api_key.clone(),
                    requested_model.to_string(),
                    self.config.timeout_secs,
                )?)));
            }
        }

        Ok(self.grok.clone())
    }
}

struct ProviderRun {
    results: Vec<ProviderSearchResult>,
    providers: Vec<String>,
    answer: Option<String>,
    tokens: u32,
}

fn extend_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !target.iter().any(|existing| existing == value) {
            target.push(value.clone());
        }
    }
}

fn format_results(results: &[ProviderSearchResult], answer: Option<&str>) -> String {
    let mut sections = Vec::new();
    if let Some(answer) = answer {
        sections.push(answer.to_string());
    }

    let sources = results
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let snippet = if result.snippet.is_empty() {
                String::new()
            } else {
                format!(" - {}", result.snippet)
            };
            format!(
                "{}. {} ({})\n   {}{}",
                index + 1,
                result.title,
                result.source,
                result.url,
                snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    sections.push(format!("Sources:\n{sources}"));
    sections.join("\n\n")
}

fn format_sub_queries(
    sub_results: &[SubQueryResult],
    merged_results: &[ProviderSearchResult],
) -> String {
    let sections = sub_results
        .iter()
        .map(|item| format!("{}\n{}", item.sub_query, item.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!("{}\n\n{}", sections, format_results(merged_results, None))
}
