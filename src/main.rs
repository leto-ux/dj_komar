use std::io::{BufRead, BufReader};
use std::process::Command;
use std::thread;
use std::time::Duration;

fn main() {
    let port_name = "/dev/ttyUSB0";
    let baud_rate = 9600;

    // just gonna panic for now, no need to do error handling yet
    let port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(3000))
        .open()
        .expect("Failed to open port");

    let mut reader = BufReader::new(port);
    let mut last_volume: Option<f32> = None;

    thread::sleep(Duration::from_millis(3000));

    loop {
        let mut line = String::new();

        match reader.read_line(&mut line) {
            Ok(_) => {
                if let Ok(value) = line.trim().parse::<u16>() {
                    // println!("Potentiometer value: {value}");
                    let volume = ((value as f32 / 1023.0) * 100.0).round() / 100.0;

                    let should_update = match last_volume {
                        Some(prev) => (volume - prev).abs() > 0.03,
                        None => true,
                    };

                    if should_update {
                        last_volume = Some(volume);

                        let output = Command::new("sh")
                            .arg("-c")
                            .arg(format!("echo raw: {volume}"))
                            .output()
                            .expect("failed to echo");

                        println!("{}", String::from_utf8_lossy(&output.stdout));

                        let _output = Command::new("sh")
                            .arg("-c")
                            .arg(format!("wpctl set-volume 33 {volume}"))
                            .output()
                            .expect("failed to set volume");
                    }
                } else {
                    println!("Received non-numeric: {}", line.trim());
                }
            }
            Err(e) => {
                eprintln!("{e}");
                break;
            }
        }
    }
}
