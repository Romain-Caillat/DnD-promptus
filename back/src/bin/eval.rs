//! `bun run eval` — run the evaluation set (`content/evals/`) against
//! the configured model, save the run in `evals/runs/` and print what
//! improved or regressed since the previous run
//! (`ai/evaluate-on-real-campaigns`).
//!
//! The model is the server's (`.env`: `OPENROUTER_API_KEY`,
//! `OPENROUTER_MODEL`); `--fake` runs the deterministic provider, to
//! try the bench itself. Each run costs real money with a real key.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use promptus_back::ai::Ai;
use promptus_back::ai::fake::FakeProvider;
use promptus_back::eval::{self, Run};

fn latest(runs: &Path) -> Option<Run> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(runs)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    files.sort();
    let text = std::fs::read_to_string(files.last()?).ok()?;
    serde_json::from_str(&text).ok()
}

#[tokio::main]
async fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let content = root.join("content");
    let runs = root.join("evals/runs");
    let fake = std::env::args().any(|a| a == "--fake");

    let cases = match eval::load_cases(&content.join("evals")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cas illisibles : {e}");
            return ExitCode::FAILURE;
        }
    };
    let fake_provider = FakeProvider::default();
    let ai = Ai::from_env();
    let provider = if fake {
        &fake_provider as &dyn promptus_back::ai::Provider
    } else {
        match ai.provider() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e} (ou lancez `bun run eval -- --fake`)");
                return ExitCode::FAILURE;
            }
        }
    };

    let previous = latest(&runs);
    let run = eval::run(provider, &cases, &content).await;
    let report = eval::report(previous.as_ref(), &run);
    println!("{report}");
    if fake {
        // The fake provider's runs say nothing about a model: not kept.
        return ExitCode::SUCCESS;
    }
    let stamp = run.at.format("%Y-%m-%dT%H-%M-%S").to_string();
    let saved = std::fs::create_dir_all(&runs)
        .and_then(|()| {
            std::fs::write(
                runs.join(format!("{stamp}.json")),
                serde_json::to_string_pretty(&run).unwrap_or_default(),
            )
        })
        .and_then(|()| std::fs::write(runs.join(format!("{stamp}.md")), &report));
    if let Err(e) = saved {
        eprintln!("impossible d'enregistrer l'évaluation : {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("enregistrée dans evals/runs/{stamp}.json");
    ExitCode::SUCCESS
}
