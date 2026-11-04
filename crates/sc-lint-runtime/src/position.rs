use syn::spanned::Spanned;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourcePosition {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

impl SourcePosition {
    pub(crate) fn start(value: &impl Spanned) -> Self {
        let position = value.span().start();
        Self {
            line: position.line,
            column: position.column,
        }
    }
    pub(crate) fn end(value: &impl Spanned) -> Self {
        let position = value.span().end();
        Self {
            line: position.line,
            column: position.column,
        }
    }
}
