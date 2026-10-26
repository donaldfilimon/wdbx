//! Hard profile ceilings; callers may only narrow them. Counts use checked arithmetic.

/// Profile ceiling or fixed grammar size: header bytes.
pub const HEADER_BYTES: u64 = 96;
/// Profile ceiling or fixed grammar size: directory entry bytes.
pub const DIRECTORY_ENTRY_BYTES: u64 = 80;
/// Profile ceiling or fixed grammar size: min components.
pub const MIN_COMPONENTS: u64 = 8;
/// Profile ceiling or fixed grammar size: max components.
pub const MAX_COMPONENTS: u64 = 32;
/// Profile ceiling or fixed grammar size: max artifact bytes.
pub const MAX_ARTIFACT_BYTES: u64 = 67_108_864;
/// Profile ceiling or fixed grammar size: max metadata bytes.
pub const MAX_METADATA_BYTES: u64 = 2_097_152;
/// Profile ceiling or fixed grammar size: max tokenizer bytes.
pub const MAX_TOKENIZER_BYTES: u64 = 1_048_576;
/// Profile ceiling or fixed grammar size: max string bytes.
pub const MAX_STRING_BYTES: u64 = 4_096;
/// Profile ceiling or fixed grammar size: max relation label bytes.
pub const MAX_RELATION_LABEL_BYTES: u64 = 64;
/// Profile ceiling or fixed grammar size: max token bytes.
pub const MAX_TOKEN_BYTES: u64 = 65_536;
/// Profile ceiling or fixed grammar size: max nodes.
pub const MAX_NODES: u64 = 128;
/// Profile ceiling or fixed grammar size: min nodes.
pub const MIN_NODES: u64 = 2;
/// Profile ceiling or fixed grammar size: max edges.
pub const MAX_EDGES: u64 = 1_024;
/// Profile ceiling or fixed grammar size: max hidden width.
pub const MAX_HIDDEN_WIDTH: u64 = 64;
/// Profile ceiling or fixed grammar size: max relations.
pub const MAX_RELATIONS: u64 = 8;
/// Profile ceiling or fixed grammar size: min vocab.
pub const MIN_VOCAB: u64 = 260;
/// Profile ceiling or fixed grammar size: max vocab.
pub const MAX_VOCAB: u64 = 4_096;
/// Profile ceiling or fixed grammar size: max merges.
pub const MAX_MERGES: u64 = 3_836;
/// Profile ceiling or fixed grammar size: max message steps.
pub const MAX_MESSAGE_STEPS: u64 = 4;
/// Profile ceiling or fixed grammar size: min sequence tokens.
pub const MIN_SEQUENCE_TOKENS: u64 = 2;
/// Profile ceiling or fixed grammar size: max sequence tokens.
pub const MAX_SEQUENCE_TOKENS: u64 = 128;
/// Profile ceiling or fixed grammar size: max generated tokens.
pub const MAX_GENERATED_TOKENS: u64 = 128;
/// Profile ceiling or fixed grammar size: max batch examples.
pub const MAX_BATCH_EXAMPLES: u64 = 4;
/// Profile ceiling or fixed grammar size: max incoming degree.
pub const MAX_INCOMING_DEGREE: u64 = 16;
/// Profile ceiling or fixed grammar size: max trainable scalars.
pub const MAX_TRAINABLE_SCALARS: u64 = 1_000_000;
/// Profile ceiling or fixed grammar size: max session bytes.
pub const MAX_SESSION_BYTES: u64 = 268_435_456;
/// Profile ceiling or fixed grammar size: max loaded and trainer bytes.
pub const MAX_LOADED_AND_TRAINER_BYTES: u64 = 536_870_912;
/// Profile ceiling or fixed grammar size: max training scalar ops.
pub const MAX_TRAINING_SCALAR_OPS: u64 = 500_000_000;
/// Profile ceiling or fixed grammar size: max loading bytes.
pub const MAX_LOADING_BYTES: u64 = 268_435_456;
/// Profile ceiling or fixed grammar size: loading overhead bytes.
pub const LOADING_OVERHEAD_BYTES: u64 = 16_777_216;
/// Profile ceiling or fixed grammar size: max tokenizer training bytes.
pub const MAX_TOKENIZER_TRAINING_BYTES: u64 = 134_217_728;
/// Profile ceiling or fixed grammar size: max training documents.
pub const MAX_TRAINING_DOCUMENTS: u64 = 4_096;
/// Profile ceiling or fixed grammar size: max document bytes.
pub const MAX_DOCUMENT_BYTES: u64 = 65_536;
/// Profile ceiling or fixed grammar size: max corpus bytes.
pub const MAX_CORPUS_BYTES: u64 = 16_777_216;
/// Profile ceiling or fixed grammar size: max trainer visits.
pub const MAX_TRAINER_VISITS: u64 = 100_000_000;
/// Profile ceiling or fixed grammar size: max encoder visits.
pub const MAX_ENCODER_VISITS: u64 = 8_000_000;
/// Profile ceiling or fixed grammar size: max encode input bytes.
pub const MAX_ENCODE_INPUT_BYTES: u64 = 65_536;
/// Profile ceiling or fixed grammar size: max examples.
pub const MAX_EXAMPLES: u64 = 65_536;
/// Profile ceiling or fixed grammar size: max source cohorts.
pub const MAX_SOURCE_COHORTS: u64 = 32;
/// Profile ceiling or fixed grammar size: max data roots.
pub const MAX_DATA_ROOTS: u64 = 1_024;
/// Profile ceiling or fixed grammar size: max licenses.
pub const MAX_LICENSES: u64 = 64;
/// Profile ceiling or fixed grammar size: writer buffer bytes.
pub const WRITER_BUFFER_BYTES: u64 = 65_536;
/// Exact learned-coordinate bound, not a configurable clamp.
pub const COORDINATE_RADIUS: f64 = 4.0;
/// ID allocation reserves the final value; growth uses checked arithmetic.
pub const MAX_STABLE_ID: u64 = u64::MAX - 1;

/// Caller policy ceilings for future bounded loading. No execution estimate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadLimits {
    /// Whole owned artifact bytes.
    pub artifact_bytes: u64,
    /// Aggregate metadata excluding explicitly classified numeric/vocabulary payloads.
    pub metadata_bytes: u64,
    /// Total tokenizer component bytes, including descriptors.
    pub tokenizer_bytes: u64,
    /// Snapshot, typed buffers, tables, copied variable buffers and overhead reserve.
    pub loading_bytes: u64,
}
impl Default for LoadLimits {
    fn default() -> Self {
        Self {
            artifact_bytes: MAX_ARTIFACT_BYTES,
            metadata_bytes: MAX_METADATA_BYTES,
            tokenizer_bytes: MAX_TOKENIZER_BYTES,
            loading_bytes: MAX_LOADING_BYTES,
        }
    }
}
impl LoadLimits {
    /// Clamp caller requests to the hard profile. Zero remains zero (refusal).
    #[must_use]
    pub fn effective(self) -> Self {
        Self {
            artifact_bytes: self.artifact_bytes.min(MAX_ARTIFACT_BYTES),
            metadata_bytes: self.metadata_bytes.min(MAX_METADATA_BYTES),
            tokenizer_bytes: self.tokenizer_bytes.min(MAX_TOKENIZER_BYTES),
            loading_bytes: self.loading_bytes.min(MAX_LOADING_BYTES),
        }
    }
}

/// Native tokenizer API ceiling for input raw token IDs (structural wrappers separate).
pub const MAX_DECODE_TOKEN_IDS: u64 = 65_536;
/// Native tokenizer API output ceiling; larger than raw input to admit large merges.
/// Together with the bounded vocabulary this stays below the package/session budget.
pub const MAX_DECODE_OUTPUT_BYTES: u64 = 67_108_864;
