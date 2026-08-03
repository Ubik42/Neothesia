use std::path::PathBuf;

fn main() {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let Some(midi_path) = args.next() else {
        usage();
    };
    let Some(score_path) = args.next() else {
        usage();
    };
    if args.next().is_some() {
        usage();
    }

    let midi = midi_file::MidiFile::new(&midi_path).unwrap_or_else(|error| {
        eprintln!("could not read MIDI: {error}");
        std::process::exit(1);
    });
    let sidecar =
        neothesia_core::library::save_score_association(&midi_path, &midi.content_id, &score_path)
            .unwrap_or_else(|error| {
                eprintln!("could not associate score: {error}");
                std::process::exit(1);
            });
    println!("{}", sidecar.display());
}

fn usage() -> ! {
    eprintln!("usage: score-associate <midi> <musicxml>");
    std::process::exit(2);
}
