//! Bounded diagnostics for host analysis; observations never admit results.
use meden_runtime::process::OutputProgress;

pub(super) fn stage(script: &str, args: &[String]) -> &'static str {
    match script {
        "prepare.sh" => "Preparing analysis tools",
        "compile.sh" => "Compiling selected sources",
        "project-atlas-prepare.sh" if args.first().is_some_and(|a| a == "prepare") => {
            "Building Lean dependencies"
        }
        "project-atlas-prepare.sh" => "Validating compiled inputs",
        "extract.sh" | "project-atlas-extract.sh" => "Extracting graph and explanations",
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
