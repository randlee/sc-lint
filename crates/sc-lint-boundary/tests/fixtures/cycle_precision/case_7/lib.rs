pub struct DecomposedMessageAdmission {
    pub tag: String,
}

impl DecomposedMessageAdmission {
    pub fn expected_tag_provenance(&self) -> String {
        self.expected_tag_provenance_for(&self.tag)
    }

    pub fn expected_tag_provenance_for(&self, tag: &str) -> String {
        tag.to_owned()
    }
}
