use regex::Regex;
use std::error::Error;
use std::fmt;
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::thread;
use std::time::Duration;

const SINK_COUNT: usize = 6;
const REVERSE_POT: bool = true;

#[derive(Debug)]
enum ParseError {
    InvalidFormat,
    InvalidPotId,
    InvalidSinkId,
    InvalidValue,
    Heartbeat,
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

fn volume_parse(line_ref: &str) -> Option<&str> {
    let re = Regex::new(r"^A\d_(\d+)$").ok()?;
    re.captures(line_ref)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
}

fn pot_parse(line: &str) -> Result<Pot, ParseError> {
    let trimmed = line.trim();

    if trimmed == "heartbeat" {
        return Err(ParseError::Heartbeat);
    }

    let id_char = line.chars().nth(1).ok_or(ParseError::InvalidPotId)?;
    let id = id_char.to_digit(10).ok_or(ParseError::InvalidPotId)?;

    let value_str = volume_parse(line.trim()).ok_or(ParseError::InvalidFormat)?;
    let value = value_str
        .trim()
        .parse::<u16>()
        .map_err(|_| ParseError::InvalidValue)?;

    Ok(Pot { id, value })
}

fn sink_name_to_id(sink_name: [&str; SINK_COUNT]) -> Result<[usize; SINK_COUNT], ParseError> {
    let mut sink_id = [0usize; SINK_COUNT];

    for i in 0..SINK_COUNT {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "pw-dump | jq '.[] | select(.info.props.\"node.name\" == \"{}\") | .id'",
                sink_name[i]
            ))
            .output()
            .expect("pw-dump failure"); // unrecoverable as ids are needed

        sink_id[i] = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<usize>()
            .map_err(|_| ParseError::InvalidSinkId)?;
    }
    Ok(sink_id)
}

fn main() {
    let sink_name: [&str; SINK_COUNT] = [
        "1. Master",
        "2. Browser",
        "3. Music",
        "4. Comms",
        "5. Games",
        "6. Others",
    ];

    let analog_port_name: [usize; SINK_COUNT] = [0, 1, 2, 3, 9, 8];

    let sink_id = sink_name_to_id(sink_name).expect("failed to get ids at startup");
    println!("{:?}", sink_id);

    let port_name = "/dev/ttyACM0";
    let baud_rate = 9600;

    // just gonna panic for now, no need to do error handling yet
    let port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(120000))
        .open()
        .expect("Failed to open port");

    let mut reader = BufReader::new(port);

    thread::sleep(Duration::from_millis(3000));

    loop {
        let mut line = String::new();

        match reader.read_line(&mut line) {
            Ok(_) => match pot_parse(&line) {
                Ok(pot) => {
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
                        // This case handles a pot ID that isn't in our `analog_port_name` array.
                        eprintln!(
                            "Warning: Received ID {} which is not configured in analog_port_name.",
                            pot.id
                        );
                    }
                }

                Err(e) => eprintln!("Failed to parse: {}", e),
            },
            Err(e) => {
                eprintln!("{e}");
                break;
            }
        }
    }
}
