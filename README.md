![Neothesia Baner](https://github.com/user-attachments/assets/383438e5-80cd-49d2-af30-85afe5d79c6b)


# Neothesia

Neothesia is a cross-platform MIDI visualizer build in Rust.
It helps people to quickly learn how to play piano.
It takes music notes from a MIDI file as an input and displays them as colorful falling blocks on a virtual piano.

> [!IMPORTANT]
> This branch is an independently maintained piano-learning fork. It is 207
> commits ahead of upstream and is under active development. The current focus
> is deliberate practice, local repertoire management, synchronized notation
> and a dependable Pianoteq workflow. It is not yet a packaged end-user
> release; build from source or use the existing local development build.

Opensource Synthesia was abandoned in favour of [closed source commercial project](https://www.synthesiagame.com/)  
The goal of this project is to bring Opensource Synthesia back to life, and make it look and work as good (or even better) than commercial Synthesia.

If you have any questions, feel free to join my Discord

[<img alt="Discord" src="https://img.shields.io/discord/273176778946641920?logo=discord&style=for-the-badge&color=%23a051ee">](https://discord.gg/sgeZuVA)

## Screenshots

![image](https://github.com/PolyMeilex/Neothesia/assets/20758186/65483bab-0b74-4fd4-90b1-fdd00508b676)

[![Video](https://github.com/PolyMeilex/Neothesia/assets/20758186/dc564433-aade-4430-b137-5f90000ae9e0)](https://youtu.be/ReE9nVuMCSE)

|![settings](https://github.com/PolyMeilex/Neothesia/assets/20758186/e38642e2-6118-4931-9964-a1df27a36db9)|![track selection](https://github.com/PolyMeilex/Neothesia/assets/20758186/2309d970-0234-45ff-a9f4-105ff08514af)|
|--|--|

[Video](https://youtu.be/ReE9nVuMCSE)

## Download

<a href="https://flathub.org/apps/details/com.github.polymeilex.neothesia"><img width="240" alt="Download on Flathub" src="https://flathub.org/assets/badges/flathub-badge-en.png"/></a>

Arch Linux (**Unofficial AUR** built from source, maintained by @zayn7lie): <https://aur.archlinux.org/packages/neothesia>

All binary releases:
[https://github.com/PolyMeilex/Neothesia/releases](https://github.com/PolyMeilex/Neothesia/releases)

## FAQ

- [FAQ](https://polymeilex.github.io/Neothesia/pages/installation.html)
- [Video encoding](https://polymeilex.github.io/Neothesia/pages/video-encoding.html)

## Development roadmap

- [Current development status](docs/development/progress.md)
- [Sustained piano-learning plan](docs/development/README.md)
- [Product and engineering roadmap](docs/development/roadmap.md)
- [Ordered backlog](docs/development/backlog.md)
- [Piano plug-in hosting (VST3 and Pianoteq)](docs/pages/plugin-hosting-roadmap.md)
- [External Pianoteq practice workflow](docs/pages/pianoteq-external-routing.md)
- [Public practice-library sources and sync](docs/pages/practice-library.md)

### What works in this fork

- wait-for-notes practice is the default and can be switched during playback;
- measure numbers, optional beat subdivisions, hand selection, loops, count-in
  and adaptive tempo support focused passage practice;
- deterministic feedback covers note accuracy, timing, hands, measures,
  dynamics, duration and pedal, with persisted sessions and recommendations;
- the local practice library supports watched folders, search, recent songs,
  favourites, a practice queue, metadata editing and missing-file repair;
- exercise mode, manual and suggested fingering, free-play recording and
  semantic UI automation are available;
- paired MusicXML/MXL scores can be aligned to MIDI, rendered through the
  optional Verovio feature, followed across pages and highlighted during
  playback;
- external MIDI output is suitable for routing into standalone Pianoteq, with
  visible route diagnostics and panic/all-notes-off handling.

### Experimental or not implemented yet

- the engraved-score renderer is a default-off feature and currently requires
  a Node/Verovio worker;
- direct in-process VST3 hosting, including loading Pianoteq as a plug-in, has
  not been implemented; standalone Pianoteq routing is the supported path;
- dense-score reading modes, physical-device soak testing, accessibility
  polish, installer/update packaging and release automation remain roadmap
  work.

## Thanks to

- [WGPU](https://wgpu.rs/)
- [Linthesia](https://github.com/linthesia/linthesia)
- [Synthesia](https://github.com/johndpope/pianogame)
