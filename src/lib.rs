#![deny(missing_docs)]

//! This is the main library interface for this project

mod config;
mod container_logging;

use crate::{config::Config, container_logging::ContainerLogging};
use anyhow::{bail, Context, Result};
use derive_builder::Builder;
use env_logger::fmt::Color;
use getset::{Getters, MutGetters};
use log::{debug, warn, LevelFilter};
use nix::{
    fcntl::{fcntl, FcntlArg, FdFlag, OFlag},
    sys::signal::{signal, SigHandler, Signal},
    unistd::{close, dup2, fork, pipe2, read, setsid, ForkResult},
};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::io::AsRawFd,
    process::exit,
    ptr,
};

const START_PIPE_ENV_KEY: &str = "_OCI_STARTPIPE";
const SYNC_PIPE_ENV_KEY: &str = "_OCI_SYNCPIPE";
const ATTACH_PIPE_ENV_KEY: &str = "_OCI_ATTACHPIPE";

#[derive(Builder, Debug, Default, Getters, MutGetters)]
#[builder(default, pattern = "owned", setter(into))]
/// Conmon is the main structure to run the OCI container monitor.
pub struct Conmon {
    #[doc = "The main conmon configuration."]
    #[getset(get, get_mut)]
    config: Config,
}

impl Conmon {
    /// Call `run` to start a new conmon instance.
    pub fn run(&mut self) -> Result<()> {
        self.init_logging().context("init logging")?;
        debug!("Set log level to {}", self.config().log_level());

        self.config_mut().validate().context("validate config")?;
        Self::unset_locale();

        let _container_logging = ContainerLogging::new(
            self.config().log_path(),
            self.config().cuuid().as_ref(),
            self.config().name().as_ref(),
            self.config().log_tag().as_ref(),
        );

        if let Err(e) = Self::set_oom("-1000") {
            warn!("Unable to adjust oom score: {}", e)
        }

        Self::set_signal_handler().context("set signal handler")?;
        let start_pipe_fd = Self::pipe_from_env(START_PIPE_ENV_KEY).context("get start pipe")?;
        if start_pipe_fd > 0 {
            // Block for an initial write to the start pipe before spawning any childred or
            // exiting, to ensure the parent can put us in the right cgroup.
            let mut buf = vec![];
            read(start_pipe_fd, &mut buf).context("read from start pipe")?;

            // If we aren't attaching in an exec session, we don't need this anymore.
            if !self.config().exec_attach() {
                close(start_pipe_fd).context("close start pipe")?;
            }
        }

        // In the non-sync case, we double-fork in order to disconnect from the parent, as we want to
        // continue in a daemon-like way
        if !self.config().sync() {
            if let ForkResult::Parent { child } = unsafe { fork()? } {
                if let Some(path) = self.config().pidfile() {
                    fs::write(path, child.to_string()).context("write conmon pidfile")?;
                }
                exit(0);
            }
        }

        // Before we fork, ensure our children will be reaped
        unsafe { libc::atexit(Self::reap_children) };

        if let Some(_socket) = self.config.sdnotify_socket() {
            unimplemented!("sd notify sockets are not implemented yet");
        }

        let sync_pipe_fd = Self::pipe_from_env(SYNC_PIPE_ENV_KEY).context("get sync pipe")?;
        let mut attach_pipe_fd = None;
        if self.config.exec_attach() {
            attach_pipe_fd = Self::pipe_from_env(ATTACH_PIPE_ENV_KEY)
                .context("get attach pipe")?
                .into();
        }

        // Disconnect stdio from parent. We need to do this, because the parent is waiting for the
        // stdout to end when the intermediate child dies
        const DEV_NULL: &str = "/dev/null";
        let dev_null_r = OpenOptions::new().read(true).open(DEV_NULL)?;
        let dev_null_w = OpenOptions::new().write(true).open(DEV_NULL)?;

        dup2(dev_null_r.as_raw_fd(), libc::STDIN_FILENO)?;
        dup2(dev_null_w.as_raw_fd(), libc::STDOUT_FILENO)?;
        dup2(dev_null_w.as_raw_fd(), libc::STDERR_FILENO)?;

        // Create a new session group
        setsid()?;

        // Set self as subreaper so we can wait for container process and return its exit code.
        if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } != 0 {
            bail!("failed to set as subreaper")
        }

        let mut workerfd_stdin = -1;
        let mut workerfd_stdout = -1;
        let mut workerfd_stderr = -1;
        let mut mainfd_stdin = -1;
        let mut mainfd_stdout = -1;

        if self.config().terminal() {
            // setup_console_socket
            unimplemented!("console socket setup is not implemented yet")
        } else {
            // Create a "fake" main fd so that we can use the same epoll code in both cases. The
            // workerfd_*s will be closed after we dup over everything. We use pipes here because
            // open(/dev/std{out,err}) will fail if we used anything else (and it wouldn't be a
            // good idea to create a new pty pair in the host).
            if self.config().stdin() {
                let stdin = pipe2(OFlag::O_CLOEXEC)?;
                mainfd_stdin = stdin.0;
                workerfd_stdin = stdin.1;

                if unsafe {
                    glib_sys::g_unix_set_fd_nonblocking(
                        mainfd_stdin,
                        glib_sys::GTRUE,
                        ptr::null_mut(),
                    )
                } == glib_sys::GFALSE
                {
                    warn!("Failed to set mainfd_stdin to non blocking")
                }
            }

            let stdout = pipe2(OFlag::O_CLOEXEC)?;
            mainfd_stdout = stdout.0;
            workerfd_stdout = stdout.1;

            // Now that we've set mainfd_stdout, we can register the ctrl_winsz_cb if we didn't set
            // it here, we'd risk attempting to run ioctl on a negative fd, and fail to resize the
            // window
        }

        Ok(())
    }

    /// Initialize the logger and set the verbosity to the provided level.
    fn init_logging(&self) -> Result<()> {
        // Set the logging verbosity via the env
        let level = self.config().log_level().to_string();
        env::set_var("RUST_LOG", level);

        // Initialize the logger with the format:
        // [YYYY-MM-DDTHH:MM:SS:MMMZ LEVEL crate::module file:LINE] MSG…
        // The file and line will be only printed when running with debug or trace level.
        let log_level = self.config.log_level();
        env_logger::builder()
            .format(move |buf, r| {
                let mut style = buf.style();
                style.set_color(Color::Black).set_intense(true);
                writeln!(
                    buf,
                    "{}{} {:<5} {}{}{} {}",
                    style.value("["),
                    buf.timestamp_millis(),
                    buf.default_styled_level(r.level()),
                    r.target(),
                    match (log_level >= LevelFilter::Debug, r.file(), r.line()) {
                        (true, Some(file), Some(line)) => format!(" {}:{}", file, line),
                        _ => "".into(),
                    },
                    style.value("]"),
                    r.args()
                )
            })
            .try_init()
            .context("init env logger")
    }

    /// Unset the locale for the current process.
    fn unset_locale() {
        unsafe { libc::setlocale(libc::LC_ALL, "".as_ptr() as *const i8) };
    }

    /// Helper to adjust the OOM score of the currently running process.
    fn set_oom(score: &str) -> Result<()> {
        File::open("/proc/self/oom_score_adj")
            .context("open oom score file")?
            .write_all(score.as_bytes())
            .context("write oom score")
    }

    /// Sets the signal handler SIGPIPE to ignore and calls `exit` on SIGTERM.
    fn set_signal_handler() -> Result<()> {
        extern "C" fn handle_exit(_: i32) {
            exit(libc::EXIT_FAILURE);
        }
        unsafe {
            signal(Signal::SIGPIPE, SigHandler::SigIgn).context("ignore SIGPIPE")?;
            signal(Signal::SIGTERM, SigHandler::Handler(handle_exit)).context("handle SIGTERM")?;
        }
        Ok(())
    }

    /// Retrieve a pipe file descriptor from the provided env key.
    fn pipe_from_env(key: &str) -> Result<i32> {
        let value = env::var(key)?.parse::<i32>().context("parse env key")?;
        fcntl(value, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC)).context("make CLOEXEC")
    }

    extern "C" fn reap_children() {
        // We need to reap any zombies (from an OCI runtime that errored) before exiting
        unsafe { while libc::waitpid(-1, ptr::null_mut(), libc::WNOHANG) > 0 {} };
    }
}
