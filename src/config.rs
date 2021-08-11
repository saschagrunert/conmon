//! Configuration related structures
use anyhow::{bail, Context, Result};
use clap::{crate_version, AppSettings, Parser};
use derive_builder::Builder;
use getset::{CopyGetters, Getters, Setters};
use log::{debug, LevelFilter};
use serde::{Deserialize, Serialize};
use std::{env, path::PathBuf};

macro_rules! prefix {
    () => {
        "CONMON_"
    };
}

#[derive(
    Builder, CopyGetters, Debug, Deserialize, Eq, Getters, Parser, PartialEq, Serialize, Setters,
)]
#[builder(default, pattern = "owned", setter(into, strip_option))]
#[serde(rename_all = "kebab-case")]
#[clap(
    after_help("More info at: https://github.com/containers/conmon"),
    version(crate_version!()),
)]
/// An OCI container runtime monitor.
pub struct Config {
    #[get_copy = "pub"]
    #[clap(
        default_value("info"),
        env(concat!(prefix!(), "LOG_LEVEL")),
        long("log-level"),
        possible_values(["trace", "debug", "info", "warn", "error", "off"]),
        value_name("LEVEL")
    )]
    /// The logging level of the application.
    log_level: LevelFilter,

    #[get_copy = "pub"]
    #[clap(
        default_value("0"),
        env(concat!(prefix!(), "API_VERSION")),
        long("api-version"),
        value_name("VERSION")
    )]
    /// API version to use.
    api_version: u8,

    #[getset(get = "pub", set)]
    #[clap(
        env(concat!(prefix!(), "BUNDLE")),
        long("bundle"),
        short('b'),
        value_name("PATH")
    )]
    /// Location of the OCI Bundle path.
    bundle: Option<PathBuf>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "CID")),
        long("cid"),
        short('c'),
        value_name("ID")
    )]
    /// Identification of Container.
    cid: String,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "PIDFILE")),
        long("conmon-pidfile"),
        short('P'),
        value_name("PATH")
    )]
    /// PID file for the initial pid inside of container.
    conmon_pidfile: Option<PathBuf>,

    #[getset(get = "pub", set)]
    #[clap(
        env(concat!(prefix!(), "CONTAINER_PIDFILE")),
        long("container-pidfile"),
        short('p'),
        value_name("PATH")
    )]
    /// PID file for the conmon process.
    container_pidfile: Option<PathBuf>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "CUUID")),
        long("cuuid"),
        short('u'),
        value_name("ID")
    )]
    /// Container UUID.
    cuuid: Option<String>,

    #[get_copy = "pub"]
    #[clap(
        conflicts_with("restore"),
        env(concat!(prefix!(), "EXEC")),
        long("exec"),
        requires("exec-process-spec"),
        short('e'),
    )]
    /// Exec a command into a running container.
    exec: bool,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "EXEC_ATTACH")),
        long("exec-attach"),
        requires("exec"),
    )]
    /// Attach to an exec session.
    exec_attach: bool,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "EXEC_PROCESS_SPEC")),
        long("exec-process-spec"),
        value_name("PATH")
    )]
    /// Path to the process spec for execution.
    exec_process_spec: Option<PathBuf>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "EXIT_COMMAND")),
        long("exit-command"),
        value_name("PATH")
    )]
    /// Path to the program to execute when the container terminates its execution.
    exit_command: Option<PathBuf>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "EXIT_COMMAND_ARG")),
        long("exit-command-arg"),
        multiple_occurrences(true),
        value_name("ARGS")
    )]
    /// Additional arg to pass to the exit command.  Can be specified multiple times.
    exit_command_arg: Vec<String>,

    #[get_copy = "pub"]
    #[clap(
        default_value("0"),
        env(concat!(prefix!(), "EXIT_DELAY")),
        long("exit-delay"),
        value_name("SEC")
    )]
    /// Delay before invoking the exit command (in seconds).
    exit_delay: u32,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "EXIT_DIR")),
        long("exit-dir"),
        value_name("PATH")
    )]
    /// Path to the directory where exit files are written.
    exit_dir: Option<PathBuf>,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "LEAVE_STDIN_OPEN")),
        long("leave-stdin-open"),
    )]
    /// Leave stdin open when attached client disconnects.
    leave_stdin_open: bool,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "LOG_PATH")),
        long("log-path"),
        multiple_occurrences(true),
        required(true),
        short('l'),
        value_name("[DRIVER:]PATH")
    )]
    /// Log file paths to specified. Can also contain colon prefixd path containing the additional
    /// log driver.
    log_path: Vec<String>,

    #[get_copy = "pub"]
    #[clap(
        default_value("-1"),
        env(concat!(prefix!(), "LOG_SIZE_MAX")),
        long("log-size-max"),
        value_name("BYTE")
    )]
    /// Maximum size of log file.
    log_size_max: i64,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "LOG_TAG")),
        long("log-tag"),
        value_name("TAG")
    )]
    /// Additional tag to use for logging.
    log_tag: Option<String>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "NAME")),
        long("name"),
        short('n'),
        value_name("NAME")
    )]
    /// Container name.
    name: Option<String>,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "NO_NEW_KEYRING")),
        long("no-new-keyring"),
    )]
    /// Do not create a new session keyring for the container.
    no_new_keyring: bool,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "NO_PIVOT")),
        long("no-pivot"),
    )]
    /// Do not use `pivot_root`.
    no_pivot: bool,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "NO_SYNC_LOG")),
        long("no-sync-log"),
    )]
    /// Do not manually call sync on logs after container shutdown.
    no_sync_log: bool,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "PERSIST_DIR")),
        long("persist-dir"),
        value_name("PATH")
    )]
    /// Persistent directory for a container that can be used for storing container data.
    persist_dir: Option<PathBuf>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "PIDFILE")),
        hidden(true),
        long("pidfile"),
        value_name("PATH")
    )]
    /// PID file (deprecated).
    pidfile: Option<PathBuf>,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "REPLACE_LISTEN_PID")),
        long("replace-listen-pid"),
    )]
    /// Replace listen pid if set for oci-runtime pid.
    replace_listen_pid: bool,

    #[get = "pub"]
    #[clap(
        conflicts_with("exec"),
        env(concat!(prefix!(), "RESTORE")),
        hidden(true),
        long("restore"),
        value_name("PATH")
    )]
    /// Restore a container from a checkpoint.
    restore: Option<PathBuf>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "RESTORE_ARG")),
        hidden(true),
        long("restore-arg"),
        multiple_occurrences(true),
        value_name("ARGS")
    )]
    /// Additional arg to pass to the restore command. Can be specified multiple times (deprecated).
    restore_arg: Vec<String>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "RUNTIME")),
        long("runtime"),
        short('r'),
        value_name("PATH")
    )]
    /// Path to store runtime data for the container.
    runtime: PathBuf,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "RUNTIME_ARG")),
        long("runtime-arg"),
        multiple_occurrences(true),
        value_name("ARGS")
    )]
    /// Additional arg to pass to the runtime. Can be specified multiple times.
    runtime_arg: Vec<String>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "RUNTIME_OPT")),
        long("runtime-opt"),
        multiple_occurrences(true),
        value_name("OPTS")
    )]
    /// Additional opts to pass to the restore or exec command. Can be specified multiple times.
    runtime_opt: Vec<String>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "SDNOTIFY_SOCKET")),
        long("sdnotify_socket"),
        value_name("PATH")
    )]
    /// Path to the host's sd-notify socket to relay messages to.
    sdnotify_socket: Option<PathBuf>,

    #[get = "pub"]
    #[clap(
        default_value("/var/run/crio"),
        env(concat!(prefix!(), "SOCKET_DIR_PATH")),
        long("socket-dir-path"),
        value_name("PATH")
    )]
    /// Location of container attach sockets.
    socket_dir_path: PathBuf,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "STDIN")),
        long("stdin"),
        short('i'),
    )]
    /// Open up a pipe to pass stdin to the container.
    stdin: bool,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "SYNC")),
        long("sync"),
    )]
    /// Keep the main conmon process as its child by only forking once.
    sync: bool,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "SYSLOG")),
        long("syslog"),
    )]
    /// Log to syslog (use with cgroupfs cgroup manager).
    syslog: bool,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "SYSTEMD_CGROUP")),
        long("systemd-cgroup"),
        short('s'),
    )]
    /// Enable systemd cgroup manager, rather then use the cgroupfs directly.
    systemd_cgroup: bool,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "TERMINAL")),
        long("terminal"),
        short('t'),
    )]
    /// Allocate a pseudo-TTY.
    terminal: bool,

    #[get_copy = "pub"]
    #[clap(
        default_value("0"),
        env(concat!(prefix!(), "TIMEOUT")),
        long("timeout"),
        short('T'),
        value_name("SEC")
    )]
    /// Kill container after specified timeout in seconds.
    timeout: u32,

    #[get_copy = "pub"]
    #[clap(
        env(concat!(prefix!(), "FULL_ATTACH")),
        long("full-attach"),
    )]
    /// Don't truncate the path to the attach socket. This option causes conmon to ignore --socket-dir-path"
    full_attach: bool,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "SECCOMP_NOTIFY_SOCKET")),
        long("seccomp-notify-socket"),
        value_name("PATH")
    )]
    /// Path to the socket where the seccomp notification fd is received.
    seccomp_notify_socket: Option<PathBuf>,

    #[get = "pub"]
    #[clap(
        env(concat!(prefix!(), "SECCOMP_NOTIFY_PLUGINS")),
        long("seccomp-notify-plugins"),
        value_name("PLUGINS")
    )]
    /// Plugins to use for managing the seccomp notifications.
    seccomp_notify_plugins: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self::parse()
    }
}

impl Config {
    /// Validate the configuration integrity.
    pub fn validate(&mut self) -> Result<()> {
        if self.api_version() < 1 && self.exec_attach() {
            bail!("attach can only be specified for a non-legacy exec session")
        }

        // The old exec API did not require cuuid
        if self.cuuid().is_none() && (!self.exec() || self.api_version() >= 1) {
            bail!("container UUID not provided, use --cuuid")
        }

        if !self.runtime().exists() {
            bail!("runtime path '{}' does not exist", self.runtime().display())
        }

        let cwd = env::current_dir().context("get current dir")?;

        // `bundle` in `exec` means we will set up the attach socket for the exec session. The
        // legacy version of exec does not need this and thus we only override an empty
        // `bundle` when we're not doing an exec.
        if self.bundle().is_none() && !self.exec() {
            let bundle = cwd.clone();
            debug!("Using default bundle path: {}", bundle.display());
            self.set_bundle(bundle.into());
        }

        if self.container_pidfile().is_none() {
            let container_pidfile = cwd.join(format!("pidfile-{}", self.cid()));
            debug!(
                "Using default container pidfile: {}",
                container_pidfile.display(),
            );
            self.set_container_pidfile(container_pidfile.into());
        }

        Ok(())
    }
}
