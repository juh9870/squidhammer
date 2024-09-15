#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum DiagnosticLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub info: miette::Report,
    pub level: DiagnosticLevel,
}
