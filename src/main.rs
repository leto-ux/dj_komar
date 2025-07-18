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
    loop {
        let mut line = String::new();

        match reader.read_line(&mut line) {
            Ok(_) => {
                if let Ok(value) = line.trim().parse::<u16>() {
                    // println!("Potentiometer value: {value}");
                    let volume = ((value as f32 / 1023.0) * 100.0).round() / 100.0;

                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(format!("wpctl set-volume 33 {volume}"))
                        .output()
                        .expect("fucked up");

                    println!("{}", String::from_utf8_lossy(&output.stdout));

                    // let output = Command::new("sh")
                    //     .arg("-c")
                    //     .arg(format!("echo volume: {:.2}", volume))
                    //     .output()
                    //     .expect("failed to execute");
                    //
                    // println!("{}", String::from_utf8_lossy(&output.stdout));
                    //
                    // let output = Command::new("sh")
                    //     .arg("-c")
                    //     .arg(format!("echo raw: {:.2}", value))
                    //     .output()
                    //     .expect("failed to execute");
                    //
                    // println!("{}", String::from_utf8_lossy(&output.stdout));
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
