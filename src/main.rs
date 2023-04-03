use std::io::{self, Write};
use std::str::FromStr;
use std::time::Duration;

use clap::{Arg, Command};

const ESC: &str = "\x1b";

#[derive(Debug)]
struct State {
    pub incoming_command: Vec<u8>,
    pub step: CommandFlow,
    pub input_size: Resolution,
    pub output_size: Resolution,
}

impl Default for State {
    fn default() -> Self {
        Self {
            incoming_command: Vec::new(),
            step: CommandFlow::Uninitialized,
            input_size: Resolution { h: 0, v: 0 },
            output_size: Resolution { h: 0, v: 0 },
        }
    }
}

#[derive(Debug)]
enum CommandFlow {
    Uninitialized,
    Reconfig,
    GotHorizontalSize,
    GotVerticalSize,
    SetHSize,
    SetVSize,
    SetHCenter,
}

#[derive(Debug)]
struct Resolution {
    pub h: u32,
    pub v: u32,
}

#[derive(PartialEq, Debug)]
pub enum ExtronResponse {
    Unknown,
    Reconfig,
    ActivePixels(u32),
    ActiveLines(u32),
    InputHSizeSet,
    InputVSizeSet,
    HorizontalCenter,
    VertialCenter,
}

fn main() -> Result<(), String> {
    let mut state = State::default();

    let matches = Command::new("Extron Scaler Control")
        .about("Scales the output to match the input size and centers it")
        .disable_version_flag(true)
        .arg(
            Arg::new("port")
                .help("The device path to a serial port")
                .use_value_delimiter(false)
                .required(true),
        )
        .arg(
            Arg::new("baud")
                .help("The baud rate to connect at: 300, 600, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200")
                .use_value_delimiter(false)
                .required(true)
                .validator(valid_baud),
        )
        .arg(
            Arg::new("output_h")
                .help("The scaler's output horizontal resolution")
                .default_value("1920")
                .use_value_delimiter(false)
                .required(false)
                .validator(valid_resolution),
        )
        .arg(
            Arg::new("output_v")
                .help("The scaler's output vertical resolution")
                .use_value_delimiter(false)
                .required(false)
                .default_value("1080")
                .validator(valid_resolution),
        )
        .get_matches();

    let port_name = matches.value_of("port").unwrap();
    let baud_rate = matches.value_of("baud").unwrap().parse::<u32>().unwrap();
    state.output_size.h = matches
        .value_of("output_h")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    state.output_size.v = matches
        .value_of("output_v")
        .unwrap()
        .parse::<u32>()
        .unwrap();

    println!("Receiving data on {} at {} baud", &port_name, &baud_rate);
    println!("Output resolution: {:?}", state.output_size);

    let port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(10))
        .open();

    match port {
        Ok(mut port) => {
            let mut serial_buf: Vec<u8> = vec![0; 1000];
            loop {
                match port.read(serial_buf.as_mut_slice()) {
                    Ok(_) => {
                        // Copy any characters that are not null or line endings.
                        for b in serial_buf
                            .iter()
                            .filter(|b| **b != 0 && **b != '\n' as u8 && **b != '\r' as u8)
                        {
                            state.incoming_command.push(*b);
                        }
                        // If the character was not a line feed, the wait for more characters
                        if serial_buf[0] != '\n' as u8 {
                            continue;
                        }
                        let response = decode_response(state.incoming_command.drain(..).collect());
                        println!("Extron response: {:?}", response);
                        update_state(response, &mut state);
                        println!("New state: {:?}", state);
                        if let Some(output) = process_state(&mut state) {
                            println!("Sending command: {}", output);
                            if let Err(e) = port.write_all(output.as_bytes()) {
                                eprintln!("{:?}", e);
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("{:?}", e),
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open \"{}\". Error: {}", port_name, e);
            ::std::process::exit(1);
        }
    }
}

fn valid_baud(val: &str) -> Result<(), String> {
    let v = val
        .parse::<u32>()
        .map_err(|_| format!("Invalid baud rate '{}' specified", val))?;

    let accept: [u32; 10] = [
        300, 600, 1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200,
    ];
    if accept.contains(&v) {
        Ok(())
    } else {
        Err("Unsupported baud rate".to_string())
    }
}

fn valid_resolution(val: &str) -> Result<(), String> {
    let v = val
        .parse::<u32>()
        .map_err(|_| format!("Invalid resolution '{}' specified", val))?;
    if v > 0 {
        Ok(())
    } else {
        Err("Resolution can't be zero".to_string())
    }
}

fn decode_response(buffer: Vec<u8>) -> ExtronResponse {
    if let Ok(command) = String::from_utf8(buffer) {
        println!("Received command: {}", command);
        if command == "Reconfig" {
            ExtronResponse::Reconfig
        } else {
            match &command[0..3] {
                "Apix" => {
                    if let Ok(pixels) = u32::from_str(&command[4..]) {
                        ExtronResponse::ActivePixels(pixels)
                    } else {
                        eprintln!("Could not decode active horizontal pixels");
                        ExtronResponse::Unknown
                    }
                }
                "Alin" => {
                    if let Ok(pixels) = u32::from_str(&command[4..]) {
                        ExtronResponse::ActiveLines(pixels)
                    } else {
                        eprintln!("Could not decode active vertical lines");
                        ExtronResponse::Unknown
                    }
                }
                "Hsiz" => ExtronResponse::InputHSizeSet,
                "Vsiz" => ExtronResponse::InputVSizeSet,
                "Hctr" => ExtronResponse::HorizontalCenter,
                "Vctr" => ExtronResponse::VertialCenter,
                _ => {
                    eprintln!("Could not decode message");
                    ExtronResponse::Unknown
                }
            }
        }
    } else {
        eprintln!("Could not decode message");
        ExtronResponse::Unknown
    }
}

fn update_state(response: ExtronResponse, state: &mut State) {
    state.incoming_command.shrink_to_fit();
    match response {
        ExtronResponse::Unknown => {
            *state = State::default();
        }
        ExtronResponse::Reconfig => {
            *state = State::default();
            state.step = CommandFlow::Reconfig;
        }
        ExtronResponse::ActivePixels(h) => {
            state.input_size.h = h;
            state.step = CommandFlow::GotHorizontalSize
        }
        ExtronResponse::ActiveLines(v) => {
            state.input_size.v = v;
            state.step = CommandFlow::GotVerticalSize
        }
        ExtronResponse::InputHSizeSet => state.step = CommandFlow::SetHSize,
        ExtronResponse::InputVSizeSet => state.step = CommandFlow::SetVSize,
        ExtronResponse::HorizontalCenter => state.step = CommandFlow::SetHCenter,
        ExtronResponse::VertialCenter => state.step = CommandFlow::Uninitialized,
    }
}

fn process_state(state: &mut State) -> Option<String> {
    match state.step {
        CommandFlow::Reconfig => {
            // Get active pixels (Width)
            Some(format!("{}APIX\r", ESC))
        }
        CommandFlow::GotHorizontalSize => {
            // Get active lines (Height)
            Some(format!("{}ALIN\r", ESC))
        }
        CommandFlow::GotVerticalSize => {
            // Now that we have Width + Height,
            // Set the scaled horizontal size
            Some(format!("{}{}HSIZ\r", ESC, &state.input_size.h))
        }
        CommandFlow::SetHSize => {
            // Set the scaled vertical size
            Some(format!("{}{}VSIZ\r", ESC, &state.input_size.v))
        }
        CommandFlow::SetVSize => {
            // Center horizontally
            let h = 10240 + (state.output_size.h / 2 - state.input_size.h / 2);
            Some(format!("{}{}HCTR\r", ESC, h))
        }
        CommandFlow::SetHCenter => {
            // Center vertically
            let v = 10240 + (state.output_size.v / 2 - state.input_size.v / 2);
            Some(format!("{}{}VCTR\r", ESC, v))
        }
        _ => None,
    }
}
