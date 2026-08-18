use tantivy::schema::{
    Field, IndexRecordOption, NumericOptions, Schema, SchemaBuilder, TextFieldIndexing,
    TextOptions, STORED, STRING,
};

pub const CODE_TOKENIZER_NAME: &str = "code_tokenizer";

pub const BOOST_QUALIFIED_NAME: f32 = 4.0;
pub const BOOST_PATH: f32 = 2.0;
pub const BOOST_COMMIT_MESSAGE: f32 = 1.2;
pub const BOOST_CHUNK_TEXT: f32 = 1.0;

/// Schema definitions and field handles for Tantivy lexical search index per D-06 and INDEX-03.
#[derive(Clone, Debug)]
pub struct TantivySchema {
    pub schema: Schema,
    pub touch_id: Field,
    pub lineage_id: Field,
    pub commit_hash: Field,
    pub qualified_name: Field,
    pub path: Field,
    pub commit_message: Field,
    pub chunk_text: Field,
}

impl Default for TantivySchema {
    fn default() -> Self {
        Self::new()
    }
}

impl TantivySchema {
    pub fn new() -> Self {
        let mut builder = SchemaBuilder::default();

        let num_options = NumericOptions::default()
            .set_indexed()
            .set_stored()
            .set_fast();
        let touch_id = builder.add_u64_field("touch_id", num_options.clone());
        let lineage_id = builder.add_u64_field("lineage_id", num_options);
        let commit_hash = builder.add_text_field("commit_hash", STRING | STORED);

        let code_indexing = TextFieldIndexing::default()
            .set_tokenizer(CODE_TOKENIZER_NAME)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let code_options = TextOptions::default()
            .set_indexing_options(code_indexing)
            .set_stored();

        let qualified_name = builder.add_text_field("qualified_name", code_options.clone());
        let path = builder.add_text_field("path", code_options);

        let text_indexing = TextFieldIndexing::default()
            .set_tokenizer("default")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_options = TextOptions::default()
            .set_indexing_options(text_indexing)
            .set_stored();

        let commit_message = builder.add_text_field("commit_message", text_options.clone());
        let chunk_text = builder.add_text_field("chunk_text", text_options);

        let schema = builder.build();

        Self {
            schema,
            touch_id,
            lineage_id,
            commit_hash,
            qualified_name,
            path,
            commit_message,
            chunk_text,
        }
    }
}

/// Document representation for inserting a function touch into Tantivy index.
#[derive(Clone, Debug)]
pub struct TantivyTouchDoc {
    pub touch_id: u64,
    pub lineage_id: u64,
    pub commit_hash: String,
    pub qualified_name: String,
    pub path: String,
    pub commit_message: String,
    pub chunk_text: String,
}
