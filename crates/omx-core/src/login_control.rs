//! Cancellable, time-bounded waiting for the official-CLI login child process.
//!
//! The menubar drives `codex login` / `claude auth login`, which open a browser
//! and block until the user finishes the OAuth callback. Closing the browser
//! leaves the child waiting forever, so a plain `command.status()` would hang
//! the (TTY-less) menubar with no way out. This module spawns the child and
//! polls it against a global cancel flag and a deadline, killing it on either.
//!
//! ponytail: single global flag, not per-login tokens. Only one login runs at a
//! time (serialized by the menubar operation lock), so one flag is enough.
//! Upgrade to a generation counter if concurrent logins ever become real.

use crate::{OpenMuxError, Result};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static CANCEL_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Default ceiling for how long a login may wait on the browser callback.
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);

/// Flip the cancel flag so an in-flight [`run_cancellable_login`] kills its
/// child and returns. Safe to call from another thread; it takes no locks.
pub fn request_login_cancel() {
    CANCEL_REQUESTED.store(true, Ordering::SeqCst);
}

/// Clear any stale cancel request. Call at the start of every login so a cancel
/// left over from a previous attempt does not abort the next one.
pub fn reset_login_cancel() {
    CANCEL_REQUESTED.store(false, Ordering::SeqCst);
}

fn cancel_requested() -> bool {
    CANCEL_REQUESTED.load(Ordering::SeqCst)
}

/// Spawn `command` and wait for it, killing the child if a cancel is requested
/// or `timeout` elapses. Returns the child's exit status on natural completion;
/// a sanitized error otherwise. `describe` names the binary for error text.
pub fn run_cancellable_login(
    command: &mut Command,
    timeout: Duration,
    describe: &str,
) -> Result<ExitStatus> {
    reset_login_cancel();
    let mut child = command
        .spawn()
        .map_err(|err| OpenMuxError::Message(format!("failed to run {describe} login: {err}")))?;

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(OpenMuxError::Message(format!(
                    "failed to wait for {describe} login: {err}"
                )));
            }
        }

        if cancel_requested() {
            let _ = child.kill();
            let _ = child.wait();
            reset_login_cancel();
            return Err(OpenMuxError::Message("login cancelled".to_string()));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(OpenMuxError::Message(
                "login timed out waiting for the browser; try again".to_string(),
            ));
        }

        std::thread::sleep(Duration::from_millis(150));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // The cancel flag is global; serialize so one test's cancel can't trip another.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn cancel_kills_a_long_running_child() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_login_cancel();
        // Cancel arrives from another thread mid-wait, the real flow: the login
        // thread is parked in run_cancellable_login while a separate FFI call
        // flips the flag. A pre-set flag would be cleared by the entry reset.
        let waiter = std::thread::spawn(|| {
            let mut command = Command::new("sleep");
            command.arg("30");
            run_cancellable_login(&mut command, LOGIN_TIMEOUT, "sleep")
        });
        std::thread::sleep(Duration::from_millis(300));
        request_login_cancel();
        match waiter.join().unwrap() {
            Err(OpenMuxError::Message(msg)) => assert_eq!(msg, "login cancelled"),
            other => panic!("expected cancellation, got {other:?}"),
        }
        // Flag is cleared for the next attempt.
        assert!(!cancel_requested());
    }

    #[test]
    fn timeout_kills_a_long_running_child() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_login_cancel();
        let mut command = Command::new("sleep");
        command.arg("30");
        let result = run_cancellable_login(&mut command, Duration::from_millis(200), "sleep");
        match result {
            Err(OpenMuxError::Message(msg)) => assert!(msg.contains("timed out"), "{msg}"),
            other => panic!("expected timeout, got {other:?}"),
        }
    }

    #[test]
    fn natural_completion_returns_status() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset_login_cancel();
        let mut command = Command::new("true");
        let status = run_cancellable_login(&mut command, LOGIN_TIMEOUT, "true").unwrap();
        assert!(status.success());
    }
}
