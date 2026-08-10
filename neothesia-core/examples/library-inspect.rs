use std::{env, path::PathBuf, process::ExitCode, time::Instant};

use neothesia_core::library::LibraryIndex;

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let Some(root) = arguments.next().map(PathBuf::from) else {
        eprintln!("Usage: library-inspect <library-root> [search terms]");
        return ExitCode::from(2);
    };
    if !root.is_dir() {
        eprintln!(
            "Library root does not exist or is not a directory: {}",
            root.display()
        );
        return ExitCode::from(2);
    }
    let query = arguments
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let started = Instant::now();
    let index = LibraryIndex::scan(std::slice::from_ref(&root));
    let matches = index.search(&query);

    println!("root={}", root.display());
    println!("elapsed_ms={}", started.elapsed().as_millis());
    println!("midi_files={}", index.midi_files_seen);
    println!("unique_songs={}", index.songs.len());
    println!("unreadable_midi={}", index.unreadable_files);
    println!("catalog_files={}", index.catalog_files_seen);
    println!("catalog_entries={}", index.catalog_entries_loaded);
    println!("invalid_catalog_entries={}", index.invalid_catalog_entries);
    println!("query={query}");
    println!("query_matches={}", matches.len());
    for song in matches.into_iter().take(10) {
        let provenance = song
            .provenance
            .as_ref()
            .map(|value| format!("{} | {} | {}", value.category, value.source, value.license))
            .unwrap_or_else(|| "no catalog provenance".into());
        println!("- {} | {provenance}", song.display_name);
    }
    ExitCode::SUCCESS
}
