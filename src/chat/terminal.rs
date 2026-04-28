use anyhow::{Context, Result};
use std::io;
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
#[cfg(unix)]
use std::thread::{self, JoinHandle};
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
const PENDING_INPUT_DRAIN_GRACE: Duration = Duration::from_millis(100);

#[cfg(unix)]
pub(super) struct PendingInputGuard {
    fd: libc::c_int,
    original: libc::termios,
    original_flags: libc::c_int,
    stop: Arc<AtomicBool>,
    drain_thread: Option<JoinHandle<()>>,
}

#[cfg(not(unix))]
pub(super) struct PendingInputGuard;

#[cfg(unix)]
impl PendingInputGuard {
    pub(super) fn new() -> Result<Self> {
        let stdin = io::stdin();
        let fd = stdin.as_raw_fd();
        let original_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if original_flags < 0 {
            return Err(io::Error::last_os_error())
                .context("Could not inspect terminal input flags");
        }

        let mut termios = unsafe {
            let mut termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut termios) != 0 {
                return Err(io::Error::last_os_error()).context("Could not read terminal settings");
            }
            termios
        };
        let original = termios;

        termios.c_lflag &= !(libc::ECHO | libc::ICANON | libc::IEXTEN | libc::ISIG);
        termios.c_cc[libc::VMIN] = 1;
        termios.c_cc[libc::VTIME] = 0;

        if unsafe { libc::tcsetattr(fd, libc::TCSAFLUSH, &termios) } != 0 {
            return Err(io::Error::last_os_error()).context("Could not block pending input");
        }

        if unsafe { libc::fcntl(fd, libc::F_SETFL, original_flags | libc::O_NONBLOCK) } < 0 {
            let _ = unsafe { libc::tcsetattr(fd, libc::TCSAFLUSH, &original) };
            return Err(io::Error::last_os_error()).context("Could not drain terminal input");
        }

        drain_nonblocking_input(fd)?;

        let stop = Arc::new(AtomicBool::new(false));
        let drain_stop = Arc::clone(&stop);
        let drain_thread = thread::spawn(move || {
            while !drain_stop.load(Ordering::Relaxed) {
                let _ = drain_nonblocking_input(fd);
                thread::sleep(Duration::from_millis(10));
            }
            let _ = drain_nonblocking_input(fd);
        });

        Ok(Self {
            fd,
            original,
            original_flags,
            stop,
            drain_thread: Some(drain_thread),
        })
    }
}

#[cfg(unix)]
impl Drop for PendingInputGuard {
    fn drop(&mut self) {
        thread::sleep(PENDING_INPUT_DRAIN_GRACE);
        self.stop.store(true, Ordering::Relaxed);
        if let Some(drain_thread) = self.drain_thread.take() {
            let _ = drain_thread.join();
        }
        let _ = drain_nonblocking_input(self.fd);
        unsafe {
            libc::tcflush(self.fd, libc::TCIFLUSH);
            libc::fcntl(self.fd, libc::F_SETFL, self.original_flags);
            libc::tcsetattr(self.fd, libc::TCSAFLUSH, &self.original);
        }
    }
}

#[cfg(not(unix))]
impl PendingInputGuard {
    pub(super) fn new() -> Result<Self> {
        Ok(Self)
    }
}

#[cfg(unix)]
pub(super) fn discard_pending_input() -> Result<()> {
    let stdin = io::stdin();
    let fd = stdin.as_raw_fd();
    drain_pending_input(fd)?;
    unsafe {
        libc::tcflush(fd, libc::TCIFLUSH);
    }
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn discard_pending_input() -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn drain_pending_input(fd: libc::c_int) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error()).context("Could not inspect terminal input flags");
    }

    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error()).context("Could not drain terminal input");
    }

    let drain_result = drain_nonblocking_input(fd);

    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags) } < 0 {
        return Err(io::Error::last_os_error()).context("Could not restore terminal input flags");
    }

    drain_result?;

    unsafe {
        libc::tcflush(fd, libc::TCIFLUSH);
    }
    Ok(())
}

#[cfg(unix)]
fn drain_nonblocking_input(fd: libc::c_int) -> Result<()> {
    let mut buf = [0_u8; 1024];
    loop {
        let read = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if read > 0 {
            continue;
        }

        if read == 0 {
            return Ok(());
        }

        let error = io::Error::last_os_error();
        match error.kind() {
            io::ErrorKind::WouldBlock => return Ok(()),
            io::ErrorKind::Interrupted => continue,
            _ => return Err(error).context("Could not drain terminal input"),
        }
    }
}
