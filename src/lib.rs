use std::thread;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Error;
use std::io::ErrorKind;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::path::Path;
use std::process::Child;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use input::event::keyboard::KeyState;
use input::event::keyboard::KeyboardEvent::Key;
use input::event::keyboard::KeyboardEventTrait;
use input::event::Event::Keyboard;
use input::{Libinput, LibinputInterface};

use libc::{O_RDONLY, O_RDWR, O_WRONLY};

use log::debug;
use nix::poll::{poll, PollFd, PollFlags};

struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<RawFd, i32> {
        match path.to_str() {
            Some(x) => debug!("PATH:{}", x),
            None => (),
        }

        OpenOptions::new()
            .custom_flags(flags)
            .read((flags & O_RDONLY != 0) | (flags & O_RDWR != 0))
            .write((flags & O_WRONLY != 0) | (flags & O_RDWR != 0))
            .open(path)
            .map(|file| file.into_raw_fd())
            .map_err(|err| err.raw_os_error().unwrap())
    }
    fn close_restricted(&mut self, fd: RawFd) {
        debug!("close key_runner");
        unsafe {
            File::from_raw_fd(fd);
        }
    }
}

//Terminate the invoked process and wait for cleanup.
pub fn terminate_child(child_process: Result<Child, Error>, wait: bool) -> bool {
    if child_process.as_ref().is_ok() {
        let tmp_child = child_process;
        let mut child: Child = tmp_child.expect("error1 no child process");
        let child_pid = child.id() as u32;
        debug!("Find the process.");
        if is_pid(child_pid) {
            debug!("Terminate the process.");
            let terminate_handle = thread::spawn(move || {
                debug!("send terminate signal");
                let mut cmd = Command::new("kill");
                cmd.args(["-15", child_pid.to_string().as_str()]);
                let mut terminate = cmd.spawn().expect("Failed to terminate process.");
                terminate.wait().expect("Terminate failed to complete.");
                debug!("command_child.wait()");
                child.wait().expect("Command failed to complete.");
            });
            if wait {
              //On sigterm wait for cleanup completion before main program exit.
              terminate_handle.join().expect("Failed to join terminate thread.");
            }
            true
        } else {
            false
        }
    } else {
        false
    }
}

//Check the pid is of an active process.
pub fn is_pid(pid: u32) -> bool {
    let mut cmd = Command::new("ps");
    cmd.args(["-hp", pid.to_string().as_str()]);
    let mut pid_number = 0;
    let ps_data = cmd.output().expect("Could not execute ps.");
    let data = ps_data.stdout;
    let data: &str = std::str::from_utf8(&data).unwrap().trim();
    if data.len() != 0 {
        let mut iter = data.split_whitespace();
        let found_pid = iter.next().expect("no_pid error");
        let conversion = u32::from_str_radix(found_pid, 10);
        if conversion.is_ok() {
            pid_number = conversion.unwrap();
        } else {
            false;
        }
    }
    pid_number == pid
}

pub fn start_process(command: &Vec<&str>) -> Result<Child, Error> {
    let mut cmd = Command::new(command[0]);
    cmd.args(&command[1..]);
    debug!("new spawn");
    let child_process = cmd.spawn();
    let child = child_process.as_ref().expect("Failed to start xdotool");
    let child_pid = child.id() as u32;
    debug!("Started process: {}", child_pid);
    child_process
}

mod params;

pub fn doit() {
    env_logger::init();
    let params = params::get_params();
    debug!(
        "Params: command:{}, key:{}",
        params.command, params.key_code
    );
    let command: Vec<&str> = params.command.as_str().split_whitespace().collect();
    let mut input: input::Libinput = Libinput::new_from_path(Interface);
    let keyboard_list: Vec<rs_input::Keyboard> = rs_input::get_keyboards();
    if keyboard_list.len() < 1 {
        panic!("no keyboard found");
    }
    let keyboard_path = &keyboard_list[0].path;
    debug!("Keyboard: {}", keyboard_path);
    let _keyboard_device = input.path_add_device(keyboard_path);

    //Holder so we know when to spawn/terminate a process for the command.
    let mut child_process: Result<Child, Error> = Err(Error::new(ErrorKind::Other, "init"));

    //set a handler to deal with a terminate signal on this process.
    let quit: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let stop = quit.clone();
    ctrlc::set_handler(move || {
        stop.store(true, Ordering::Relaxed);
    })
    .expect("Could not handle sigint!!");

    //A file descriptor polled to determine if unprocessed input events exist.
    let pollfd = PollFd::new(input.as_raw_fd(), PollFlags::POLLIN);

    while !quit.load(Ordering::Relaxed) {
        while poll(&mut [pollfd], -1).is_ok() {
            input.dispatch().unwrap();
            for event in &mut input {
                if let Keyboard(keyboard_event) = &event {
                    if let Key(keyboard_key_event) = keyboard_event {
                        let key_event = keyboard_key_event as &dyn KeyboardEventTrait;
                        if key_event.key_state() == KeyState::Pressed {
                            let keycode: u32 = key_event.key();
                            debug!("key:{}", keycode);

                            if keycode == (params.key_code) {
                                if child_process.as_ref().is_ok() {
                                    if terminate_child(child_process, false) {
                                        child_process = Err(Error::new(ErrorKind::Other, "init"));
                                    } else {
                                        child_process = start_process(&command);
                                    }
                                } else {
                                    debug!("No process so start it.");
                                    child_process = start_process(&command);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    terminate_child(child_process, true);
}

#[cfg(test)]
pub mod tests {
    #[test]
    fn test_is_pid() {
        use std::process::Command;
        let mut cmd = Command::new("sleep");
        cmd.arg("1s");
        let proc = cmd.spawn();
        let pid = proc.expect("Could not execute test sleep process").id();
        let ret: bool = crate::is_pid(pid);
        assert!(ret);
    }
}
