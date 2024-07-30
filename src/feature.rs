#[derive(Clone)]
pub struct Feature {
    feature_vector: Vec<f32>,
    source_file: String,
    id: Option<i64>,
}

impl Feature {
    pub fn new(feature_vector: Vec<f32>, source_file: String, id: Option<i64>) -> Self {
        Self {
            feature_vector,
            source_file,
            id,
        }
    }

    pub fn feature_vector(&self) -> &[f32] {
        &self.feature_vector
    }

    pub fn source_file(&self) -> &str {
        &self.source_file
    }

    pub fn id(&self) -> &Option<i64> {
        &self.id
    }
}
