use std::path::Path;

use anyhow::{Context, Result};
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::schema::*;
use tantivy::{Index, IndexWriter};

use crate::connectors::NormalizedConversation;

const SCHEMA_VERSION: &str = "v1";

pub struct TantivyIndex {
    pub index: Index,
    writer: IndexWriter,
    fields: Fields,
}

struct Fields {
    agent: Field,
    workspace: Field,
    source_path: Field,
    msg_idx: Field,
    created_at: Field,
    title: Field,
    content: Field,
}

impl TantivyIndex {
    pub fn open_or_create(path: &Path) -> Result<Self> {
        let dir = MmapDirectory::open(path).or_else(|_| {
            std::fs::create_dir_all(path)?;
            Ok(MmapDirectory::open(path)?)
        })?;

        let schema = build_schema();
        let index = Index::open_or_create(dir, schema.clone())
            .with_context(|| format!("open/create tantivy index at {}", path.display()))?;
        let writer = index
            .writer(50_000_000)
            .with_context(|| "create index writer")?;

        let fields = Fields {
            agent: schema.get_field("agent").unwrap(),
            workspace: schema.get_field("workspace").unwrap(),
            source_path: schema.get_field("source_path").unwrap(),
            msg_idx: schema.get_field("msg_idx").unwrap(),
            created_at: schema.get_field("created_at").unwrap(),
            title: schema.get_field("title").unwrap(),
            content: schema.get_field("content").unwrap(),
        };

        Ok(Self {
            index,
            writer,
            fields,
        })
    }

    pub fn add_conversation(&mut self, conv: &NormalizedConversation) -> Result<()> {
        for msg in &conv.messages {
            let mut doc = Document::new();
            doc.add_text(self.fields.agent, &conv.agent_slug);
            if let Some(ws) = &conv.workspace {
                doc.add_text(self.fields.workspace, ws.to_string_lossy());
            }
            doc.add_text(self.fields.source_path, conv.source_path.to_string_lossy());
            doc.add_u64(self.fields.msg_idx, msg.idx as u64);
            if let Some(ts) = msg.created_at.or(conv.started_at) {
                doc.add_i64(self.fields.created_at, ts);
            }
            if let Some(title) = &conv.title {
                doc.add_text(self.fields.title, title);
            }
            doc.add_text(self.fields.content, &msg.content);
            self.writer.add_document(doc)?;
        }
        Ok(())
    }

    pub fn commit(&mut self) -> Result<()> {
        self.writer.commit()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn search_sample(&self, query: &str) -> Result<Vec<(f32, tantivy::DocAddress)>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        let fields = vec![self.fields.title, self.fields.content];
        let qb = tantivy::query::QueryParser::for_index(&self.index, fields);
        let q = qb.parse_query(query)?;
        let top = searcher.search(&q, &TopDocs::with_limit(10))?;
        Ok(top)
    }
}

fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("agent", TEXT | STORED);
    schema_builder.add_text_field("workspace", TEXT | STORED);
    schema_builder.add_text_field("source_path", STORED);
    schema_builder.add_u64_field("msg_idx", INDEXED | STORED);
    schema_builder.add_i64_field("created_at", INDEXED | STORED);
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("content", TEXT | STORED);
    schema_builder.build()
}

pub fn index_dir(base: &Path) -> Result<std::path::PathBuf> {
    let dir = base.join("index").join(SCHEMA_VERSION);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
