use clap::{Command, Arg};

pub struct Params {
    pub command: String,
    pub key_code: u32,
    pub device : Option<String>
}

fn is_string(command: &str) -> Result<(), String> {
  if command.len() > 0 {
    Ok(())
  } else {
    Err(String::from("A command is required."))
  }
}

fn is_integer(int: &str) -> Result<(), String> {
    match u32::from_str_radix(int, 10) {
        Ok(_) => Ok(()),
        Err(_) => Err(String::from("An unsigned integer is required.")),
    }
}

pub fn get_params() -> Params {
    let command = Arg::new("command")
        .short('c')
        .long("command")
        .takes_value(true)
        .required(true)
        .validator(is_string)
        .help("Command to be triggered.");

    let key = Arg::new("key")
        .short('k')
        .long("key")
        .takes_value(true)
        .required(true)
        .validator(is_integer)
        .help("The key code of the trigger key. eg 61 for F3");

    let device = Arg::new("device")
        .short('d')
        .long("device")
        .takes_value(true)
        .required(false)
        .help("Configure the /sys/class/input device path.");

    let args = [command, key, device];
    let arg_matches = Command::new("key_binder")
        .long_about("Bind a command to a key. A subsequent key press kills the process spawned by the command, or spawns a new process if the process has terminated.")
        .args(&args)
        .get_matches();

    Params {
        command: String::from(arg_matches.value_of("command").unwrap()),
        key_code: u32::from_str_radix(arg_matches.value_of("key")
            .unwrap(), 10).unwrap(),
        device: {
          match arg_matches.value_of("device") {
            Some(s) => Some(String::from(s)),
            None => None
          }
        }
    }
}


