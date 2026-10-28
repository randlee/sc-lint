pub struct LocalCapability {
    pub header: String,
}

impl LocalCapability {
    pub fn matches_header(&self, candidate: &str) -> bool {
        self.parse_base64url(candidate) == self.header.len()
    }

    pub fn parse_base64url(&self, input: &str) -> usize {
        input.len()
    }
}
