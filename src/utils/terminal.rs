use std::os::fd;

use rustix::termios;

pub fn terminal_width<Fd: fd::AsFd>(fd: Fd) -> Result<usize, String> {
    if !termios::isatty(&fd) {
        Err("Unable to get terminal width: Stream is not a tty".to_owned())
    } else {
        match termios::tcgetwinsize(fd) {
            Ok(s) => Ok(s.ws_col as usize),
            Err(e) => Err(format!("Unable to get terminal width: {}", e)),
        }
    }
}
