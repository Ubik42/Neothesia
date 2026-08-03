use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use crate::{NeothesiaEvent, scene::DebugUiHarness};
use winit::event_loop::EventLoopProxy;

const ADDRESS_ENV: &str = "NEOTHESIA_DEBUG_DRIVER_ADDR";
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_COMMAND_BYTES: u64 = 4_096;

pub struct DebugDriver {
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl DebugDriver {
    pub fn start_from_env(proxy: EventLoopProxy<NeothesiaEvent>) -> Result<Option<Self>, String> {
        let Some(address) = std::env::var_os(ADDRESS_ENV) else {
            return Ok(None);
        };
        let address = address
            .to_str()
            .ok_or_else(|| format!("{ADDRESS_ENV} must be valid Unicode"))?
            .parse::<SocketAddr>()
            .map_err(|err| format!("invalid {ADDRESS_ENV}: {err}"))?;
        validate_address(address)?;

        let listener = TcpListener::bind(address)
            .map_err(|err| format!("cannot bind debug driver at {address}: {err}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|err| format!("cannot configure debug driver: {err}"))?;
        let local_address = listener
            .local_addr()
            .map_err(|err| format!("cannot inspect debug driver address: {err}"))?;

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = thread::Builder::new()
            .name("neothesia-debug-driver".into())
            .spawn(move || run(listener, proxy, worker_stop))
            .map_err(|err| format!("cannot start debug driver: {err}"))?;

        log::info!("Debug UI driver listening on {local_address}");
        Ok(Some(Self {
            stop,
            worker: Some(worker),
        }))
    }
}

fn validate_address(address: SocketAddr) -> Result<(), String> {
    if address.ip().is_loopback() {
        Ok(())
    } else {
        Err(format!("{ADDRESS_ENV} must use a loopback address"))
    }
}

impl Drop for DebugDriver {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(listener: TcpListener, proxy: EventLoopProxy<NeothesiaEvent>, stop: Arc<AtomicBool>) {
    let harness = DebugUiHarness::new(proxy);
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => handle_connection(stream, &harness),
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(err) => {
                log::warn!("Debug UI driver accept failed: {err}");
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, harness: &DebugUiHarness) {
    let _ = stream.set_read_timeout(Some(RESPONSE_TIMEOUT));
    let _ = stream.set_write_timeout(Some(RESPONSE_TIMEOUT));

    let mut line = String::new();
    let read_result = BufReader::new(&stream)
        .take(MAX_COMMAND_BYTES + 1)
        .read_line(&mut line);
    let response = match read_result {
        Ok(0) => error_response("empty-command"),
        Ok(_) if line.len() as u64 > MAX_COMMAND_BYTES => error_response("command-too-long"),
        Err(_) => error_response("read-failed"),
        Ok(_) => execute(parse_command(line.trim_end()), harness),
    };

    let _ = stream.write_all(response.as_bytes());
}

#[derive(Debug, Eq, PartialEq)]
enum DriverCommand<'a> {
    Action(&'a str),
    Midi { channel: u8, note: u8, velocity: u8 },
    Snapshot,
    Exit,
}

fn parse_command(line: &str) -> Result<DriverCommand<'_>, &'static str> {
    if line == "SNAPSHOT" {
        return Ok(DriverCommand::Snapshot);
    }
    if line == "EXIT" {
        return Ok(DriverCommand::Exit);
    }
    if let Some(values) = line.strip_prefix("MIDI ") {
        let values: Vec<_> = values.split_ascii_whitespace().collect();
        if let [channel, note, velocity] = values.as_slice()
            && let (Ok(channel), Ok(note), Ok(velocity)) = (
                channel.parse::<u8>(),
                note.parse::<u8>(),
                velocity.parse::<u8>(),
            )
            && channel <= 15
            && note <= 127
            && velocity <= 127
        {
            return Ok(DriverCommand::Midi {
                channel,
                note,
                velocity,
            });
        }
        return Err("bad-midi");
    }
    if let Some(id) = line.strip_prefix("ACTION ")
        && !id.is_empty()
        && !id.contains(char::is_whitespace)
    {
        return Ok(DriverCommand::Action(id));
    }
    Err("bad-command")
}

fn execute(command: Result<DriverCommand<'_>, &'static str>, harness: &DebugUiHarness) -> String {
    match command {
        Ok(DriverCommand::Action(id)) => match harness.activate(id, RESPONSE_TIMEOUT) {
            Some(accepted) => format!(r#"{{"ok":true,"accepted":{accepted}}}"#) + "\n",
            None => error_response("timeout"),
        },
        Ok(DriverCommand::Midi {
            channel,
            note,
            velocity,
        }) => match harness.midi_note(channel, note, velocity, RESPONSE_TIMEOUT) {
            Some(accepted) => format!(r#"{{"ok":true,"accepted":{accepted}}}"#) + "\n",
            None => error_response("timeout"),
        },
        Ok(DriverCommand::Snapshot) => match harness.snapshot(RESPONSE_TIMEOUT) {
            Some(snapshot) => format!(
                concat!(
                    r#"{{"ok":true,"snapshot":{{"#,
                    r#""wait_for_notes":{},"adaptive_tempo":{},"hands":{},"#,
                    r#""loop_active":{},"loop_start_measure":{},"loop_end_measure":{},"#,
                    r#""counting_in":{},"paused":{},"completion_tab":{},"#,
                    r#""matched_notes":{},"wrong_notes":{},"missed_notes":{},"#,
                    r#""required_notes":{},"required_note_pitches":{},"input_latency_ms":{},"#,
                    r#""fingerings_available":{},"fingerings_enabled":{},"#,
                    r#""fingering_count":{},"manual_fingering_count":{},"#,
                    r#""fingering_crossing_count":{},"fingering_editor_active":{},"#,
                    r#""suggested_finger":{},"suggested_fingering_count":{},"#,
                    r#""suggestion_confidence_percent":{},"#,
                    r#""score_artifact_ready":{},"score_synchronization_ready":{},"#,
                    r#""score_cached_pages":{},"#,
                    r#""score_focused_page":{},"score_texture_page":{},"#,
                    r#""score_texture_width":{},"score_texture_height":{},"#,
                    r#""score_visible":{}"#,
                    "}}}}\n"
                ),
                snapshot.wait_for_notes,
                snapshot.adaptive_tempo,
                json_string(snapshot.hands.map(|hands| hands.label())),
                snapshot.loop_active,
                json_number(snapshot.loop_start_measure),
                json_number(snapshot.loop_end_measure),
                snapshot.counting_in,
                snapshot.paused,
                json_string(snapshot.completion_tab),
                snapshot.matched_notes,
                snapshot.wrong_notes,
                snapshot.missed_notes,
                snapshot.required_notes,
                json_u8_array(&snapshot.required_note_pitches),
                snapshot.input_latency_ms,
                snapshot.fingerings_available,
                snapshot.fingerings_enabled,
                snapshot.fingering_count,
                snapshot.manual_fingering_count,
                snapshot.fingering_crossing_count,
                snapshot.fingering_editor_active,
                json_number(snapshot.suggested_finger),
                snapshot.suggested_fingering_count,
                json_number(snapshot.suggestion_confidence_percent),
                snapshot.score_artifact_ready,
                snapshot.score_synchronization_ready,
                snapshot.score_cached_pages,
                json_number(snapshot.score_focused_page),
                json_number(snapshot.score_texture_page),
                json_number(snapshot.score_texture_width),
                json_number(snapshot.score_texture_height),
                snapshot.score_visible,
            ),
            None => r#"{"ok":true,"snapshot":null}"#.to_owned() + "\n",
        },
        Ok(DriverCommand::Exit) => {
            if harness.shutdown(RESPONSE_TIMEOUT) {
                r#"{"ok":true}"#.to_owned() + "\n"
            } else {
                error_response("timeout")
            }
        }
        Err(error) => error_response(error),
    }
}

fn json_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".into(), |value| format!("\"{value}\""))
}

fn json_number(value: Option<usize>) -> String {
    value.map_or_else(|| "null".into(), |value| value.to_string())
}

fn json_u8_array(values: &[u8]) -> String {
    let values: Vec<_> = values.iter().map(u8::to_string).collect();
    format!("[{}]", values.join(","))
}

fn error_response(error: &str) -> String {
    format!(r#"{{"ok":false,"error":"{error}"}}"#) + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use neothesia_core::practice::PracticeHands;

    #[test]
    fn protocol_parser_accepts_only_bounded_semantic_commands() {
        assert_eq!(parse_command("SNAPSHOT"), Ok(DriverCommand::Snapshot));
        assert_eq!(parse_command("EXIT"), Ok(DriverCommand::Exit));
        assert_eq!(
            parse_command("ACTION practice.player.wait"),
            Ok(DriverCommand::Action("practice.player.wait"))
        );
        assert_eq!(
            parse_command("MIDI 0 60 100"),
            Ok(DriverCommand::Midi {
                channel: 0,
                note: 60,
                velocity: 100,
            })
        );
        assert_eq!(parse_command("MIDI 16 60 100"), Err("bad-midi"));
        assert_eq!(parse_command("MIDI 0 128 100"), Err("bad-midi"));
        assert_eq!(parse_command("ACTION"), Err("bad-command"));
        assert_eq!(
            parse_command("ACTION practice.player.wait extra"),
            Err("bad-command")
        );
    }

    #[test]
    fn snapshot_values_have_stable_json_scalars() {
        assert_eq!(json_string(None), "null");
        assert_eq!(json_number(None), "null");
        assert_eq!(json_number(Some(12)), "12");
        assert_eq!(json_u8_array(&[60, 64, 67]), "[60,64,67]");
        assert_eq!(json_string(Some(PracticeHands::Right.label())), "\"Right\"");
        assert_eq!(
            error_response("bad-command"),
            "{\"ok\":false,\"error\":\"bad-command\"}\n"
        );
    }

    #[test]
    fn driver_address_must_be_loopback() {
        assert!(validate_address("127.0.0.1:32123".parse().unwrap()).is_ok());
        assert!(validate_address("[::1]:32123".parse().unwrap()).is_ok());
        assert!(validate_address("0.0.0.0:32123".parse().unwrap()).is_err());
        assert!(validate_address("192.168.1.20:32123".parse().unwrap()).is_err());
    }
}
