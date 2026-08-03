use std::{env, fs, process::ExitCode};

use neothesia_core::score_view::VerovioManifest;

fn main() -> ExitCode {
    let arguments: Vec<_> = env::args().skip(1).collect();
    if arguments.is_empty() || arguments.len() % 2 != 0 {
        eprintln!("usage: verovio-manifest-inspect <manifest> <source-sha256> [...]");
        return ExitCode::FAILURE;
    }
    for pair in arguments.chunks_exact(2) {
        let json = match fs::read_to_string(&pair[0]) {
            Ok(json) => json,
            Err(error) => {
                eprintln!("{}: {error}", pair[0]);
                return ExitCode::FAILURE;
            }
        };
        match VerovioManifest::parse_and_validate(&json, &pair[1]) {
            Ok(manifest) => {
                let root = std::path::Path::new(&pair[0])
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."));
                if let Err(error) = manifest.validate_page_files(root) {
                    eprintln!("{}: {error}", pair[0]);
                    return ExitCode::FAILURE;
                }
                println!(
                    "{}\t{}\t{}\t{}",
                    pair[0],
                    manifest.renderer_version,
                    manifest.page_count,
                    manifest.notes.len()
                );
            }
            Err(error) => {
                eprintln!("{}: {error}", pair[0]);
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
