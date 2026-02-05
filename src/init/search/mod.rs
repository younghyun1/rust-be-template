use std::collections::HashSet;
use std::path::Path;
use std::sync::RwLock;

use tantivy::{
    Index, IndexReader, IndexSettings, IndexWriter, TantivyDocument,
    collector::{Count, MultiCollector, TopDocs},
    directory::MmapDirectory,
    query::{BooleanQuery, Occur, PhrasePrefixQuery, Query, QueryParser, TermQuery},
    schema::{
        Field, FieldType, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing,
        TextOptions, Value,
    },
};
use tracing::{info, warn};
use uuid::Uuid;

/// Disk-persisted search index for blog posts using Tantivy.
/// Indexes post titles and tags for fast full-text search.
/// Maintains coherence with the database cache.
pub struct PostSearchIndex {
    index: Index,
    reader: IndexReader,
    writer: RwLock<IndexWriter>,
    index_path: Option<std::path::PathBuf>,
    // Schema fields
    post_id_field: Field,
    title_field: Field,
    tags_field: Field,
}

impl PostSearchIndex {
    /// Build the schema used by the index.
    fn build_schema() -> (Schema, Field, Field, Field) {
        let mut schema_builder = Schema::builder();

        // Post ID stored as string for retrieval
        let post_id_field = schema_builder.add_text_field("post_id", STRING | STORED);

        // Title field - indexed for full-text search
        let text_field_indexing = TextFieldIndexing::default()
            .set_tokenizer("default")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_options = TextOptions::default()
            .set_indexing_options(text_field_indexing)
            .set_stored();
        let title_field = schema_builder.add_text_field("title", text_options);

        // Tags field - indexed as individual terms for exact matching
        let tags_field = schema_builder.add_text_field("tags", STRING | STORED);

        let schema = schema_builder.build();
        (schema, post_id_field, title_field, tags_field)
    }

    /// Create a new in-memory search index (no persistence).
    pub fn new_in_memory() -> anyhow::Result<Self> {
        let (schema, post_id_field, title_field, tags_field) = Self::build_schema();

        let index = Index::create_in_ram(schema);
        let writer = index.writer(50_000_000)?;
        let reader = index.reader()?;

        Ok(Self {
            index,
            reader,
            writer: RwLock::new(writer),
            index_path: None,
            post_id_field,
            title_field,
            tags_field,
        })
    }

    /// Open or create a disk-persisted search index.
    /// If the directory doesn't exist, it will be created.
    /// If the index exists but is corrupted, it will be recreated.
    pub fn open_or_create<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let (schema, post_id_field, title_field, tags_field) = Self::build_schema();

        // Ensure directory exists
        if !path.exists() {
            std::fs::create_dir_all(path)?;
            info!(path = %path.display(), "Created search index directory");
        }

        // Try to open existing index, create new if it doesn't exist or is corrupted
        let index = match MmapDirectory::open(path) {
            Ok(dir) => {
                match Index::open(dir) {
                    Ok(idx) => {
                        info!(path = %path.display(), "Opened existing search index");
                        idx
                    }
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "Failed to open index, creating new one");
                        // Clear the directory and create fresh
                        Self::clear_directory(path)?;
                        let dir = MmapDirectory::open(path)?;
                        Index::create(dir, schema.clone(), IndexSettings::default())?
                    }
                }
            }
            Err(e) => {
                warn!(path = %path.display(), error = %e, "Failed to open directory, creating new index");
                Self::clear_directory(path)?;
                let dir = MmapDirectory::open(path)?;
                Index::create(dir, schema.clone(), IndexSettings::default())?
            }
        };

        let writer = index.writer(50_000_000)?;
        let reader = index.reader()?;

        Ok(Self {
            index,
            reader,
            writer: RwLock::new(writer),
            index_path: Some(path.to_path_buf()),
            post_id_field,
            title_field,
            tags_field,
        })
    }

    /// Clear a directory of all files (used when recreating a corrupted index).
    fn clear_directory(path: &Path) -> anyhow::Result<()> {
        if path.exists() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    std::fs::remove_file(&path)?;
                }
            }
        }
        Ok(())
    }

    /// Get all post IDs currently in the index.
    pub fn get_indexed_post_ids(&self) -> anyhow::Result<HashSet<Uuid>> {
        let searcher = self.reader.searcher();
        let mut post_ids = HashSet::new();

        for segment_reader in searcher.segment_readers() {
            let store_reader = segment_reader.get_store_reader(1)?;
            for doc_id in segment_reader.doc_ids_alive() {
                if let Ok(doc) = store_reader.get::<TantivyDocument>(doc_id)
                    && let Some(post_id_value) = doc.get_first(self.post_id_field)
                    && let Some(post_id_str) = post_id_value.as_str()
                    && let Ok(uuid) = Uuid::parse_str(post_id_str)
                {
                    post_ids.insert(uuid);
                }
            }
        }

        Ok(post_ids)
    }

    /// Check if the index is coherent with a set of expected post IDs.
    /// Returns (missing_from_index, extra_in_index).
    pub fn check_coherence(
        &self,
        expected_post_ids: &HashSet<Uuid>,
    ) -> anyhow::Result<(Vec<Uuid>, Vec<Uuid>)> {
        let indexed_ids = self.get_indexed_post_ids()?;

        let missing: Vec<Uuid> = expected_post_ids
            .difference(&indexed_ids)
            .copied()
            .collect();

        let extra: Vec<Uuid> = indexed_ids.difference(expected_post_ids).copied().collect();

        Ok((missing, extra))
    }

    /// Index a single post. Call commit() after batch operations.
    pub fn index_post(&self, post_id: Uuid, title: &str, tags: &[String]) -> anyhow::Result<()> {
        let mut doc = TantivyDocument::new();
        doc.add_text(self.post_id_field, post_id.to_string());
        doc.add_text(self.title_field, title);

        // Add each tag as a separate field value for exact term matching
        for tag in tags {
            doc.add_text(self.tags_field, tag.to_lowercase());
        }

        let writer = self
            .writer
            .write()
            .map_err(|e| anyhow::anyhow!("Writer lock poisoned: {}", e))?;
        writer.add_document(doc)?;

        Ok(())
    }

    /// Remove a post from the index by its ID.
    pub fn remove_post(&self, post_id: Uuid) -> anyhow::Result<()> {
        let term = tantivy::Term::from_field_text(self.post_id_field, &post_id.to_string());
        let writer = self
            .writer
            .write()
            .map_err(|e| anyhow::anyhow!("Writer lock poisoned: {}", e))?;
        writer.delete_term(term);
        Ok(())
    }

    /// Commit pending changes to the index and persist to disk.
    pub fn commit(&self) -> anyhow::Result<()> {
        let mut writer = self
            .writer
            .write()
            .map_err(|e| anyhow::anyhow!("Writer lock poisoned: {}", e))?;
        writer.commit()?;
        // Reload reader to see committed changes
        self.reader.reload()?;
        Ok(())
    }

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
        let fetch_limit = limit.saturating_add(offset);
        let mut collectors = MultiCollector::new();
        let top_docs_handle = collectors.add_collector(TopDocs::with_limit(fetch_limit));
        let count_handle = collectors.add_collector(Count);
        let mut multi_fruit = searcher.search(query, &collectors)?;

        let total_matches = count_handle.extract(&mut multi_fruit);
        let top_docs = top_docs_handle.extract(&mut multi_fruit);

        let mut results = Vec::with_capacity(limit.min(top_docs.len()));
        for (_score, doc_address) in top_docs.into_iter().skip(offset).take(limit) {
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

    /// Rebuild the entire index from a list of posts.
    /// Clears existing index and re-indexes all posts.
    pub fn rebuild_index<'a, I>(&self, posts: I) -> anyhow::Result<usize>
    where
        I: Iterator<Item = (Uuid, &'a str, &'a [String])>,
    {
        // Clear the index
        {
            let mut writer = self
                .writer
                .write()
                .map_err(|e| anyhow::anyhow!("Writer lock poisoned: {}", e))?;
            writer.delete_all_documents()?;
            writer.commit()?;
        }

        let mut count = 0;
        for (post_id, title, tags) in posts {
            self.index_post(post_id, title, tags)?;
            count += 1;
        }

        self.commit()?;
        info!(posts_indexed = count, "Search index rebuilt");

        Ok(count)
    }

    /// Incrementally sync the index with a set of posts.
    /// Adds missing posts and removes extra posts.
    /// More efficient than full rebuild when only a few posts differ.
    pub fn sync_with_posts<'a, I>(&self, posts: I) -> anyhow::Result<(usize, usize)>
    where
        I: Iterator<Item = (Uuid, &'a str, &'a [String])>,
    {
        let posts_vec: Vec<_> = posts.collect();
        let expected_ids: HashSet<Uuid> = posts_vec.iter().map(|(id, _, _)| *id).collect();

        let (missing, extra) = self.check_coherence(&expected_ids)?;

        // Remove extra posts
        for post_id in &extra {
            self.remove_post(*post_id)?;
        }

        // Add missing posts
        let missing_set: HashSet<Uuid> = missing.iter().copied().collect();
        for (post_id, title, tags) in &posts_vec {
            if missing_set.contains(post_id) {
                self.index_post(*post_id, title, tags)?;
            }
        }

        if !missing.is_empty() || !extra.is_empty() {
            self.commit()?;
            info!(
                added = missing.len(),
                removed = extra.len(),
                "Search index synchronized"
            );
        }

        Ok((missing.len(), extra.len()))
    }

    /// Update a post in the index (remove old, add new) and commit immediately.
    pub fn update_post(&self, post_id: Uuid, title: &str, tags: &[String]) -> anyhow::Result<()> {
        self.remove_post(post_id)?;
        self.index_post(post_id, title, tags)?;
        self.commit()?;
        Ok(())
    }

    /// Add a new post to the index and commit immediately.
    /// Use this for single post additions to ensure disk persistence.
    pub fn add_post_and_commit(
        &self,
        post_id: Uuid,
        title: &str,
        tags: &[String],
    ) -> anyhow::Result<()> {
        self.index_post(post_id, title, tags)?;
        self.commit()?;
        Ok(())
    }

    /// Remove a post from the index and commit immediately.
    /// Use this for single post deletions to ensure disk persistence.
    pub fn remove_post_and_commit(&self, post_id: Uuid) -> anyhow::Result<()> {
        self.remove_post(post_id)?;
        self.commit()?;
        Ok(())
    }

    /// Get the index path if disk-persisted, None if in-memory.
    pub fn index_path(&self) -> Option<&Path> {
        self.index_path.as_deref()
    }

    /// Get the number of documents in the index.
    pub fn num_docs(&self) -> u64 {
        self.reader.searcher().num_docs()
    }
}
