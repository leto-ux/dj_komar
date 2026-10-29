use regex::Regex;
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

const REVERSE_POT: bool = true;

#[derive(Deserialize)]
struct Config {
    sinks: Vec<SinkConfig>,
}

#[derive(Deserialize)]
struct SinkConfig {
    name: String,
    pin: usize,
}

#[derive(Debug)]
enum ParseError {
    InvalidFormat,
    InvalidPotId,
    InvalidSinkId,
    InvalidValue,
}

impl Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

struct Pot {
    id: u32,
    value: u16,
}

enum Message {
    Pot(Pot),
    Mute(bool), // true = muted
    Heartbeat,
}

fn parse_line(line: &str) -> Result<Message, ParseError> {
    let trimmed = line.trim();

    match trimmed {
        "heartbeat" => return Ok(Message::Heartbeat),
        "MUTE_ON" => return Ok(Message::Mute(true)),
        "MUTE_OFF" => return Ok(Message::Mute(false)),
        _ => {}
    }

    let re = Regex::new(r"^A(\d+)_(\d+)$").map_err(|_| ParseError::InvalidFormat)?;
    let caps = re.captures(trimmed).ok_or(ParseError::InvalidFormat)?;

    let id = caps
        .get(1)
        .ok_or(ParseError::InvalidPotId)?
        .as_str()
        .parse::<u32>()
        .map_err(|_| ParseError::InvalidPotId)?;

    let value = caps
        .get(2)
        .ok_or(ParseError::InvalidValue)?
        .as_str()
        .parse::<u16>()
        .map_err(|_| ParseError::InvalidValue)?;

    Ok(Message::Pot(Pot { id, value }))
}

fn load_config() -> Config {
    let config_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| {
            let local = PathBuf::from("config.toml");
            if local.exists() {
                Some(local)
            } else {
                Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.toml"))
            }
        })
        .expect("Failed to determine config path");

    let content = fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Failed to read config file {:?}: {}", config_path, e));

    toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse config file {:?}: {}", config_path, e))
}

fn sink_name_to_id(sinks: &[SinkConfig]) -> Result<Vec<usize>, ParseError> {
    let mut sink_ids = Vec::with_capacity(sinks.len());

    for sink in sinks {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "pw-dump | jq '.[] | select(.info.props.\"node.name\" == \"{}\") | .id'",
                sink.name
            ))
            .output()
            .expect("pw-dump failure"); // unrecoverable as ids are needed

        let id = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<usize>()
            .map_err(|_| ParseError::InvalidSinkId)?;

        sink_ids.push(id);
    }
    Ok(sink_ids)
}

fn main() {
    let config = load_config();
    let analog_port_name: Vec<usize> = config.sinks.iter().map(|s| s.pin).collect();

    let sink_id = sink_name_to_id(&config.sinks).expect("failed to get ids at startup");
    println!("{:?}", sink_id);

    let port_name = "/dev/ttyACM0";
    let baud_rate = 9600;

    // just gonna panic for now, no need to do error handling yet
    let mut port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(120000))
        .open()
        .expect("Failed to open port");

    thread::sleep(Duration::from_millis(3000));

    let pins_cmd = format!(
        "PINS:{}\n",
        analog_port_name
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    port.write_all(pins_cmd.as_bytes())
        .expect("Failed to send pin configuration to Arduino");
    port.flush().expect("Failed to flush serial port");

    let mut reader = BufReader::new(port);

    loop {
        let mut line = String::new();

        match reader.read_line(&mut line) {
            Ok(_) => match parse_line(&line) {
                Ok(Message::Pot(pot)) => {
                    let mut volume = ((pot.value as f32 / 1023.0) * 100.0).round() / 100.0;

                    if REVERSE_POT {
                        volume = 1.0 - volume;
                    }

                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(format!(
                            "echo parsed: A{}_{} volume: {}",
                            pot.id, pot.value, volume
                        ))
                        .output()
                        .expect("failed to echo");

                    println!("{}", String::from_utf8_lossy(&output.stdout));

                    // I have no clue how does this line work exactly
                    if let Some(sink_index) =
                        analog_port_name.iter().position(|&p| p == pot.id as usize)
                    {
                        let target_sink_id = sink_id[sink_index];

                        let output = Command::new("sh")
                            .arg("-c")
                            .arg(format!("wpctl set-volume {} {:.2}", target_sink_id, volume))
                            .output()
                            .expect("failed to set volume");

                        // print any errors from wpctl
                        if !output.stderr.is_empty() {
                            eprintln!(
                                "wpctl error for sink {}: {}",
                                target_sink_id,
                                String::from_utf8_lossy(&output.stderr).trim()
                            );
                        }
                    } else {
                        eprintln!(
                            "Warning: Received ID {} which is not configured in analog_port_name.",
                            pot.id
                        );
                    }
                }

                Ok(Message::Mute(is_muted)) => {
                    if let Some(&master_sink_id) = sink_id.first() {
                        let mute_flag = if is_muted { "1" } else { "0" };

                        println!("{}", if is_muted { "mute on" } else { "mute off" });

                        let output = Command::new("sh")
                            .arg("-c")
                            .arg(format!("wpctl set-mute {} {}", master_sink_id, mute_flag))
                            .output()
                            .expect("Failed to set mute");

                        // prints stderr if it has anything in it
                        if !output.stderr.is_empty() {
                            eprintln!(
                                "wpctl mute rror for sink {}: {}",
                                master_sink_id,
                                String::from_utf8_lossy(&output.stderr).trim()
                            );
                        }
                    }
                }

                Ok(Message::Heartbeat) => {}

                Err(e) => eprintln!("Failed to parse: {}", e),
            },
            Err(e) => {
                eprintln!("{e}");
                break;
            }
        }
    }
}
