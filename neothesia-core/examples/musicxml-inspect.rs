use std::{collections::BTreeMap, env, ffi::OsStr, path::PathBuf, process::ExitCode};

use neothesia_core::musicxml::{ScoreEvent, import_musicxml_file};

fn main() -> ExitCode {
    let mut verbose = false;
    let paths: Vec<PathBuf> = env::args_os()
        .skip(1)
        .filter_map(|argument| {
            if argument == OsStr::new("--verbose") {
                verbose = true;
                None
            } else {
                Some(PathBuf::from(argument))
            }
        })
        .collect();
    if paths.is_empty() {
        eprintln!("usage: cargo run -p neothesia-core --example musicxml-inspect -- <score>...");
        return ExitCode::from(2);
    }

    let mut failed = false;
    println!("file\tparts\tmeasures\tnotes\ttuplet-notes\tdirections\twarnings\ttitle");
    for path in paths {
        match import_musicxml_file(&path) {
            Ok(score) => {
                let measures = score
                    .parts
                    .iter()
                    .map(|part| part.measures.len())
                    .sum::<usize>();
                let mut notes = 0;
                let mut tuplet_notes = 0;
                let mut directions = 0;
                for event in score
                    .parts
                    .iter()
                    .flat_map(|part| &part.measures)
                    .flat_map(|measure| &measure.events)
                {
                    match event {
                        ScoreEvent::Note(note) => {
                            notes += 1;
                            tuplet_notes += usize::from(note.time_modification.is_some());
                        }
                        ScoreEvent::Direction(_) => directions += 1,
                    }
                }
                println!(
                    "{}\t{}\t{measures}\t{notes}\t{tuplet_notes}\t{directions}\t{}\t{}",
                    path.display(),
                    score.parts.len(),
                    score.warnings.len(),
                    score.title.as_deref().unwrap_or("")
                );
                if verbose {
                    for warning in score.warnings {
                        println!(
                            "  warning\t{}\t{}",
                            warning.location,
                            warning.message.replace('\t', " ")
                        );
                    }
                } else {
                    let mut summary = BTreeMap::new();
                    for warning in score.warnings {
                        *summary.entry(warning.message).or_insert(0_usize) += 1;
                    }
                    for (message, count) in summary {
                        println!("  warning-summary\t{count}\t{}", message.replace('\t', " "));
                    }
                }
            }
            Err(error) => {
                failed = true;
                eprintln!("{}\tERROR\t{error}", path.display());
            }
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
