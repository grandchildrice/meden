//! Bounded diagnostics for host analysis; observations never admit results.
use meden_runtime::process::OutputProgress;

pub(super) fn stage(script: &str, _args: &[String]) -> &'static str {
    match script {
        "prepare.sh" => "Preparing analysis tools",
        "compile.sh" => "Compiling selected sources",
        "project-atlas-query.sh" => "Building and resolving Lean inputs",
        "project-atlas-prepare.sh" => "Validating compiled inputs",
        "extract.sh" | "project-atlas-extract.sh" => "Extracting graph and explanations",
        "search.sh" | "project-search.sh" => "Searching for counterexamples",
        _ => "Resolving Lean modules",
    }
}

pub(super) fn diagnostic(script: &str, output: &OutputProgress) -> OutputProgress {
    OutputProgress {
        // JSON protocols are admitted separately. They are not build logs.
        stdout: if matches!(script, "prepare.sh" | "compile.sh") {
            output.stdout.clone()
        } else {
            String::new()
        },
        stderr: output.stderr.clone(),
        truncated: output.truncated,
    }
}
