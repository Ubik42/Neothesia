use std::path::PathBuf;

use crate::{
    scene::menu_scene::{MsgFn, on_async},
    song::Song,
    utils::BoxFuture,
};

use super::UiState;

pub fn open_midi_file_picker(data: &mut UiState) -> BoxFuture<MsgFn> {
    data.is_loading = true;
    on_async(open_midi_file_picker_fut(), |res, data, ctx| {
        if let Some((midi, path)) = res {
            ctx.config.set_last_opened_song(Some(path));
            let mut song = Song::new(midi);
            song.apply_saved_setup(ctx);
            data.song = Some(song);
            super::state::play(data, ctx);
        }
        data.is_loading = false;
    })
}

pub fn open_saved_midi(
    data: &mut UiState,
    path: PathBuf,
    expected_content_id: String,
) -> BoxFuture<MsgFn> {
    data.is_loading = true;
    on_async(load_midi_path(path.clone()), move |res, data, ctx| {
        match res {
            Some(midi) if midi.content_id == expected_content_id => {
                ctx.config.set_last_opened_song(Some(path));
                let mut song = Song::new(midi);
                song.apply_saved_setup(ctx);
                data.song = Some(song);
                data.library_message = None;
                super::state::play(data, ctx);
            }
            Some(_) => {
                data.library_message =
                    Some("That file is a different MIDI; the saved setup was not changed.".into());
            }
            None => {
                data.library_message =
                    Some("The saved MIDI could not be opened. Choose Locate to repair it.".into());
            }
        }
        data.is_loading = false;
    })
}

pub fn locate_saved_midi(data: &mut UiState, expected_content_id: String) -> BoxFuture<MsgFn> {
    data.is_loading = true;
    on_async(open_midi_file_picker_fut(), move |res, data, ctx| {
        match res {
            Some((midi, path)) if midi.content_id == expected_content_id => {
                ctx.config.set_last_opened_song(Some(path));
                let mut song = Song::new(midi);
                song.apply_saved_setup(ctx);
                data.song = Some(song);
                data.library_message = None;
                super::state::play(data, ctx);
            }
            Some(_) => {
                data.library_message =
                    Some("That file is a different MIDI; choose the original piece.".into());
            }
            None => {}
        }
        data.is_loading = false;
    })
}

async fn load_midi_path(path: PathBuf) -> Option<midi_file::MidiFile> {
    crate::utils::task::thread::spawn("saved-midi-loader".into(), move || {
        midi_file::MidiFile::new(path).ok()
    })
    .join()
    .await
    .ok()
    .flatten()
}

async fn open_midi_file_picker_fut() -> Option<(midi_file::MidiFile, PathBuf)> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("midi", &["mid", "midi"])
        .pick_file()
        .await;

    if let Some(file) = file {
        log::info!("File path = {:?}", file.path());

        let thread = crate::utils::task::thread::spawn("midi-loader".into(), move || {
            let midi = midi_file::MidiFile::new(file.path());

            if let Err(e) = &midi {
                log::error!("{e}");
            } else {
                log::info!("MIDI loaded successfully");
            }

            midi.map(|midi| (midi, file.path().to_path_buf())).ok()
        });

        thread.join().await.ok().flatten()
    } else {
        log::info!("User canceled dialog");
        None
    }
}
