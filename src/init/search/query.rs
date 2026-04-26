use tantivy::{
    TantivyDocument,
    collector::{Count, TopDocs},
    query::{BooleanQuery, Occur, PhrasePrefixQuery, Query, QueryParser, TermQuery},
    schema::{FieldType, IndexRecordOption, Value},
};
use uuid::Uuid;

use super::PostSearchIndex;

impl PostSearchIndex {
    /// Search posts by title using full-text search.
    /// Returns up to `limit` matching post IDs.
    pub fn search_by_title(&self, query_str: &str, limit: usize) -> anyhow::Result<Vec<Uuid>> {
        Ok(self.search_by_title_paged(query_str, 0, limit)?.0)
    }

    /// Search posts by title with pagination support.
    /// Returns (post_ids, total_matches).
    pub fn search_by_title_paged(
        &self,
        query_str: &str,
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<(Vec<Uuid>, usize)> {
        let query = self.build_title_query(query_str)?;
        self.collect_post_ids(&*query, offset, limit)
    }

    /// Search posts by title and tags (all tags must match).
    pub fn search_by_title_and_tags(
        &self,
        query_str: &str,
        tags: &[String],
        limit: usize,
    ) -> anyhow::Result<Vec<Uuid>> {
        Ok(self
            .search_by_title_and_tags_paged(query_str, tags, 0, limit)?
            .0)
    }

    /// Search posts by title and tags with pagination support (all tags must match).
    /// Returns (post_ids, total_matches).
    pub fn search_by_title_and_tags_paged(
        &self,
        query_str: &str,
        tags: &[String],
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<(Vec<Uuid>, usize)> {
        let title_query = self.build_title_query(query_str)?;
        let tag_queries = self.build_tag_queries(tags);

        let mut clauses = Vec::with_capacity(1 + tag_queries.len());
        clauses.push((Occur::Must, title_query));
        for tag_query in tag_queries {
            clauses.push((Occur::Must, tag_query));
        }

        let query = BooleanQuery::new(clauses);
        self.collect_post_ids(&query, offset, limit)
    }

    fn tokenize_title_query(&self, query_str: &str) -> anyhow::Result<Vec<String>> {
        let schema = self.index.schema();
        let field_entry = schema.get_field_entry(self.title_field);
        let text_options = match field_entry.field_type() {
            FieldType::Str(text_options) => text_options,
            _ => {
                return Err(anyhow::anyhow!(
                    "Title field is not a text field; cannot tokenize query"
                ));
            }
        };
        let indexing_options = text_options
            .get_indexing_options()
            .ok_or_else(|| anyhow::anyhow!("Title field is not indexed; cannot tokenize query"))?;
        let tokenizer_name = indexing_options.tokenizer();
        let mut text_analyzer = self
            .index
            .tokenizers()
            .get(tokenizer_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tokenizer: {}", tokenizer_name))?;
        let mut tokens = Vec::new();
        let mut token_stream = text_analyzer.token_stream(query_str);
        token_stream.process(&mut |token| {
            if !token.text.is_empty() {
                tokens.push(token.text.to_string());
            }
        });
        Ok(tokens)
    }

    fn build_title_query(&self, query_str: &str) -> anyhow::Result<Box<dyn tantivy::query::Query>> {
        let query_parser = QueryParser::for_index(&self.index, vec![self.title_field]);
        if query_str.split_whitespace().count() == 1 {
            let tokens = self.tokenize_title_query(query_str)?;
            if tokens.len() == 1 {
                let term = tantivy::Term::from_field_text(self.title_field, &tokens[0]);
                return Ok(Box::new(PhrasePrefixQuery::new(vec![term])));
            }
        }
        Ok(query_parser.parse_query(query_str)?)
    }

    fn build_tag_queries(&self, tags: &[String]) -> Vec<Box<dyn tantivy::query::Query>> {
        tags.iter()
            .map(|tag| {
                let term = tantivy::Term::from_field_text(self.tags_field, &tag.to_lowercase());
                Box::new(TermQuery::new(term, IndexRecordOption::Basic))
                    as Box<dyn tantivy::query::Query>
            })
            .collect()
    }

    fn collect_post_ids(
        &self,
        query: &dyn Query,
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<(Vec<Uuid>, usize)> {
        let searcher = self.reader.searcher();
        if limit == 0 {
            let total_matches = searcher.search(query, &Count)?;
            return Ok((Vec::new(), total_matches));
        }

        let (top_docs, total_matches): (Vec<(f32, tantivy::DocAddress)>, usize) = searcher.search(
            query,
            &(
                TopDocs::with_limit(limit)
                    .and_offset(offset)
                    .order_by_score(),
                Count,
            ),
        )?;

        let mut results = Vec::with_capacity(limit.min(top_docs.len()));
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(post_id_value) = retrieved_doc.get_first(self.post_id_field)
                && let Some(post_id_str) = post_id_value.as_str()
                && let Ok(uuid) = Uuid::parse_str(post_id_str)
            {
                results.push(uuid);
            }
        }

        Ok((results, total_matches))
    }

    /// Search posts by exact tag match.
    /// Returns up to `limit` matching post IDs.
    pub fn search_by_tag(&self, tag: &str, limit: usize) -> anyhow::Result<Vec<Uuid>> {
        Ok(self.search_by_tag_paged(tag, 0, limit)?.0)
    }

    /// Search posts by exact tag match with pagination support.
    /// Returns (post_ids, total_matches).
    pub fn search_by_tag_paged(
        &self,
        tag: &str,
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<(Vec<Uuid>, usize)> {
        // Use exact term query for tags (lowercased)
        let term = tantivy::Term::from_field_text(self.tags_field, &tag.to_lowercase());
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        self.collect_post_ids(&query, offset, limit)
    }

    /// Search posts by multiple tags (all tags must match).
    pub fn search_by_tags(&self, tags: &[String], limit: usize) -> anyhow::Result<Vec<Uuid>> {
        Ok(self.search_by_tags_paged(tags, 0, limit)?.0)
    }

    /// Search posts by multiple tags with pagination support (all tags must match).
    /// Returns (post_ids, total_matches).
    pub fn search_by_tags_paged(
        &self,
        tags: &[String],
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<(Vec<Uuid>, usize)> {
        if tags.is_empty() {
            return Ok((Vec::new(), 0));
        }

        let tag_queries = self.build_tag_queries(tags);
        let clauses = tag_queries
            .into_iter()
            .map(|q| (Occur::Must, q))
            .collect::<Vec<_>>();
        let query = BooleanQuery::new(clauses);

        self.collect_post_ids(&query, offset, limit)
    }
}
