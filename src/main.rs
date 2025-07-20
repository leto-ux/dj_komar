use regex::Regex;
use std::error::Error;
use std::fmt;
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
enum ParseError {
    InvalidFormat,
    InvalidId,
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

    let id_char = line.chars().nth(1).ok_or(ParseError::InvalidId)?;
    let id = id_char.to_digit(10).ok_or(ParseError::InvalidId)?;

    let value_str = volume_parse(line.trim()).ok_or(ParseError::InvalidFormat)?;
    let value = value_str
        .trim()
        .parse::<u16>()
        .map_err(|_| ParseError::InvalidValue)?;

    Ok(Pot { id, value })
}

fn main() {
    let port_name = "/dev/ttyUSB0";
    let baud_rate = 9600;

    // just gonna panic for now, no need to do error handling yet
    let port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(30000))
        .open()
        .expect("Failed to open port");

    let mut reader = BufReader::new(port);

    thread::sleep(Duration::from_millis(3000));

    loop {
        let mut line = String::new();

        match reader.read_line(&mut line) {
            Ok(_) => match pot_parse(&line) {
                Ok(pot) => {
                    let volume = ((pot.value as f32 / 1023.0) * 100.0).round() / 100.0;

                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(format!("echo parsed: A{}_{}", pot.id, pot.value))
                        .output()
                        .expect("failed to echo");

                    println!("{}", String::from_utf8_lossy(&output.stdout));

                    // let output = Command::new("sh")
                    //     .arg("-c")
                    //     .arg(format!("wpctl set-volume 33 {volume}"))
                    //     .output()
                    //     .expect("failed to set volume");
                    //
                    // println!("{}", String::from_utf8_lossy(&output.stderr)); //just in case it errors out
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
