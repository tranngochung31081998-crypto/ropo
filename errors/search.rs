use anyhow::Result;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter, TantivyDocument};

use super::types::ErrorEntry;

/// Hybrid search for error memory: BM25 (tantivy) + simple embedding similarity
pub struct ErrorSearch {
    index: Index,
    schema: Schema,
    #[allow(dead_code)]
    data_dir: String,
}

impl ErrorSearch {
    pub fn new(data_dir: &str) -> Result<Self> {
        let schema = Self::build_schema();
        let index_path = Path::new(data_dir).join("error_index");
        std::fs::create_dir_all(&index_path)?;

        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(&index_path)?
        } else {
            Index::create_in_dir(&index_path, schema.clone())?
        };

        Ok(Self {
            index,
            schema,
            data_dir: data_dir.to_string(),
        })
    }

    fn build_schema() -> Schema {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("description", TEXT | STORED);
        schema_builder.add_text_field("context", TEXT | STORED);
        schema_builder.add_text_field("solution", TEXT | STORED);
        schema_builder.add_text_field("error_type", STRING | STORED);
        schema_builder.add_text_field("tags", TEXT | STORED);
        schema_builder.add_text_field("all_text", TEXT);
        schema_builder.add_u64_field("frequency", STORED);
        schema_builder.add_bytes_field("embedding", STORED);
        schema_builder.add_date_field("timestamp", STORED);
        schema_builder.build()
    }

    /// Index a single error entry
    pub fn index(&self, entry: &ErrorEntry) -> Result<()> {
        let id_field = self.schema.get_field("id").unwrap();
        let title_field = self.schema.get_field("title").unwrap();
        let desc_field = self.schema.get_field("description").unwrap();
        let ctx_field = self.schema.get_field("context").unwrap();
        let solution_field = self.schema.get_field("solution").unwrap();
        let error_type_field = self.schema.get_field("error_type").unwrap();
        let tags_field = self.schema.get_field("tags").unwrap();
        let all_text_field = self.schema.get_field("all_text").unwrap();
        let freq_field = self.schema.get_field("frequency").unwrap();
        let timestamp_field = self.schema.get_field("timestamp").unwrap();

        let all_text = format!(
            "{} {} {} {} {} {}",
            entry.title,
            entry.description,
            entry.context,
            entry.solution,
            entry.error_type,
            entry.tags.join(" ")
        );

        let now = chrono::Utc::now();
        let ts_secs = now.timestamp();
        let ts = tantivy::DateTime::from_timestamp_secs(ts_secs);

        let index_doc = doc!(
            id_field => entry.id.clone(),
            title_field => entry.title.clone(),
            desc_field => entry.description.clone(),
            ctx_field => entry.context.clone(),
            solution_field => entry.solution.clone(),
            error_type_field => entry.error_type.to_string(),
            tags_field => entry.tags.join(", "),
            all_text_field => all_text,
            freq_field => entry.frequency as u64,
            timestamp_field => ts,
        );

        let mut writer: IndexWriter<TantivyDocument> = self.index.writer(50_000_000)?;
        writer.add_document(index_doc)?;
        writer.commit()?;
        Ok(())
    }

    /// Re-index (update) an entry
    pub fn reindex(&self, entry: &ErrorEntry) -> Result<()> {
        let id_field = self.schema.get_field("id").unwrap();
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        let query = tantivy::query::TermQuery::new(
            tantivy::Term::from_field_text(id_field, &entry.id),
            IndexRecordOption::Basic,
        );
        let top_docs = searcher.search(&query, &TopDocs::with_limit(1))?;
        if !top_docs.is_empty() {
            let mut writer: IndexWriter<TantivyDocument> = self.index.writer(50_000_000)?;
            writer.delete_term(tantivy::Term::from_field_text(id_field, &entry.id));
            writer.commit()?;
        }
        self.index(entry)
    }

    /// Find similar entries by title similarity
    pub fn find_similar(&self, title: &str, threshold: f64) -> Result<Vec<ErrorEntry>> {
        let results = self.search(title, 10)?;
        let similar: Vec<ErrorEntry> = results
            .into_iter()
            .filter(|(_score, entry)| {
                let sim = strsim(title, &entry.title);
                sim >= threshold
            })
            .map(|(_score, entry)| entry)
            .collect();
        Ok(similar)
    }

    /// Hybrid search using BM25
    pub fn hybrid_search(&self, query: &str, limit: usize) -> Result<Vec<ErrorEntry>> {
        let results = self.search(query, limit)?;
        Ok(results.into_iter().map(|(_score, entry)| entry).collect())
    }

    /// Full-text search
    fn search(&self, query_str: &str, limit: usize) -> Result<Vec<(f64, ErrorEntry)>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        let all_text_field = self.schema.get_field("all_text").unwrap();
        let id_field = self.schema.get_field("id").unwrap();
        let title_field = self.schema.get_field("title").unwrap();
        let desc_field = self.schema.get_field("description").unwrap();
        let ctx_field = self.schema.get_field("context").unwrap();
        let solution_field = self.schema.get_field("solution").unwrap();
        let error_type_field = self.schema.get_field("error_type").unwrap();
        let tags_field = self.schema.get_field("tags").unwrap();
        let freq_field = self.schema.get_field("frequency").unwrap();
        let timestamp_field = self.schema.get_field("timestamp").unwrap();

        let query_parser = QueryParser::for_index(&self.index, vec![all_text_field]);
        let query = query_parser.parse_query(query_str)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, doc_addr) in top_docs {
            let doc: TantivyDocument = searcher.doc::<TantivyDocument>(doc_addr)?;

            let id = doc.get_first(id_field).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let title = doc.get_first(title_field).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let description = doc.get_first(desc_field).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let context = doc.get_first(ctx_field).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let solution = doc.get_first(solution_field).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let resolved = !solution.is_empty();
            let error_type_str = doc.get_first(error_type_field).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let tags_str = doc.get_first(tags_field).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let freq = doc.get_first(freq_field).and_then(|v| v.as_u64()).unwrap_or(0) as u32;

            let tags: Vec<String> = tags_str.split(", ").map(|s| s.to_string()).filter(|s| !s.is_empty()).collect();

            let _ts = doc.get_first(timestamp_field).and_then(|v| v.as_datetime());

            let entry = ErrorEntry {
                id,
                error_type: super::types::parse_error_type_str(&error_type_str),
                title,
                description,
                context,
                solution,
                code_snippet: None,
                stack_trace: None,
                timestamp: String::new(),
                last_seen: String::new(),
                frequency: freq,
                resolved,
                related_errors: Vec::new(),
                tags,
            };

            results.push((score as f64, entry));
        }

        Ok(results)
    }

    /// Search by error type
    pub fn search_by_type(&self, error_type: &super::types::ErrorType, limit: usize) -> Result<Vec<ErrorEntry>> {
        let error_type_field = self.schema.get_field("error_type").unwrap();
        let term = tantivy::Term::from_field_text(error_type_field, &error_type.to_string());
        let query = tantivy::query::TermQuery::new(term, IndexRecordOption::Basic);

        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut entries = Vec::new();
        for (_score, doc_addr) in top_docs {
            let doc: TantivyDocument = searcher.doc::<TantivyDocument>(doc_addr)?;
            let id_field = self.schema.get_field("id").unwrap();
            if let Some(id_val) = doc.get_first(id_field) {
                if let Some(id) = id_val.as_str() {
                    let mut e = ErrorEntry::new(error_type.clone(), "", "", "");
                    e.id = id.to_string();
                    entries.push(e);
                }
            }
        }

        Ok(entries)
    }
}

/// Simple string similarity (cosine of character bigrams)
fn strsim(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let a_bigrams: Vec<[char; 2]> = a
        .to_lowercase()
        .chars()
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| [w[0], w[1]])
        .collect();

    let b_bigrams: Vec<[char; 2]> = b
        .to_lowercase()
        .chars()
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| [w[0], w[1]])
        .collect();

    if a_bigrams.is_empty() || b_bigrams.is_empty() {
        return 0.0;
    }

    let intersection = a_bigrams.iter().filter(|bg| b_bigrams.contains(bg)).count();
    let union = a_bigrams.len() + b_bigrams.len() - intersection;

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}
