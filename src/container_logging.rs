//! Container logging related implementations

use anyhow::{bail, Context, Result};
use getset::{Getters, Setters};
use log::{debug, warn};
use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
    str::FromStr,
};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Getters)]
/// ContainerLogging is the structure used for everything around logging.
pub struct ContainerLogging {
    #[get]
    /// Selected log drivers.
    drivers: Vec<Driver>,

    #[get]
    /// Log files if required.
    files: Vec<File>,
}

#[derive(AsRefStr, Clone, Debug, Eq, EnumString, PartialEq)]
#[strum(serialize_all = "kebab-case")]
/// Available logging drivers.
pub enum Driver {
    /// Kubernetes file based logging.
    K8sFile(PathBuf),

    /// Journald based logging.
    Journald(ContainerFields),

    /// No logging.
    Off,

    /// No logging.
    Null,

    /// No logging.
    None,
}

#[derive(Clone, Debug, Default, Eq, Getters, PartialEq, Setters)]
pub struct ContainerFields {
    #[getset(get, set)]
    id: String,

    #[getset(get, set)]
    id_full: String,

    #[getset(get, set)]
    tag: Option<String>,

    #[getset(get, set)]
    name: Option<String>,
}

impl ContainerLogging {
    /// Create a new container logging instance.
    pub fn new<T: AsRef<str>>(
        log_paths: &[String],
        cuuid: Option<T>,
        name: Option<T>,
        tag: Option<T>,
    ) -> Result<Self> {
        debug!("Configuring container logging");

        let mut drivers: Vec<Driver> = vec![];
        let mut files: Vec<File> = vec![];

        for log_path in log_paths {
            let mut driver = Self::parse_log_path(log_path)?;
            match driver {
                Driver::Off | Driver::Null | Driver::None => continue,
                Driver::K8sFile(ref path) => {
                    if tag.is_some() {
                        warn!("Ignoring k8s-file log tag because of missing support");
                    }

                    files.push(
                        OpenOptions::new()
                            .append(true)
                            .create(true)
                            .write(true)
                            .open(path)
                            .context("open log file path")?,
                    );
                }
                Driver::Journald(ref mut fields) => {
                    const TRUNC_ID_LEN: usize = 12;
                    let cuuid: &str = cuuid.as_ref().context("no cuuid provided")?.as_ref();
                    if cuuid.len() < TRUNC_ID_LEN {
                        bail!("container ID must be longer than 12 characters")
                    }
                    let short_cuuid = Self::truncate(cuuid, TRUNC_ID_LEN);

                    fields.set_id(format!("CONTAINER_ID={}", short_cuuid));
                    fields.set_id_full(format!("CONTAINER_ID_FULL={}", cuuid));
                    fields.set_tag(
                        tag.as_ref()
                            .map(|x| format!("CONTAINER_TAG={}", x.as_ref())),
                    );
                    fields.set_name(
                        name.as_ref()
                            .map(|x| format!("CONTAINER_NAME={}", x.as_ref())),
                    );
                }
            }
            drivers.push(driver);
        }

        Ok(Self { drivers, files })
    }

    /// truncate a string slice to its maximums provided characters.
    fn truncate(s: &str, max_chars: usize) -> &str {
        match s.char_indices().nth(max_chars) {
            None => s,
            Some((idx, _)) => &s[..idx],
        }
    }

    /// Parses a logging driver from the provided `log_path`.
    ///
    /// `log_path` can either be a ':' delimited string containing:
    /// <DRIVER_NAME>:<PATH_NAME> or <PATH_NAME>
    /// in the case of no colon, the driver will be kubernetes log file,
    /// in the case the log driver is 'journald', the <PATH_NAME> is ignored.
    //
    // Errors if <DRIVER_NAME> isn't a variant of `Driver`.
    fn parse_log_path(log_path: &str) -> Result<Driver> {
        let splitted = log_path.split(':').collect::<Vec<_>>();
        let driver_or_path = *splitted.get(0).context("no driver provided")?;
        let maybe_driver = Driver::from_str(driver_or_path);

        Ok(if splitted.len() > 1 {
            match maybe_driver.context("convert log driver")? {
                Driver::K8sFile(_) => {
                    let path = *splitted.get(1).context("no path provided")?;
                    if path.is_empty() {
                        bail!("logging path cannot be empty");
                    }
                    Driver::K8sFile(path.into())
                }
                k => k,
            }
        } else {
            match maybe_driver {
                Ok(d) => d,
                // Fallback for using k8s file and assuming a path
                Err(_) => Driver::K8sFile(driver_or_path.into()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_log_path() -> Result<()> {
        struct Tc {
            input: &'static str,
            should_error: bool,
            expected: Option<Driver>,
        }
        let test_cases = vec![
            Tc {
                input: "/some/path",
                should_error: false,
                expected: Driver::K8sFile("/some/path".into()).into(),
            },
            Tc {
                input: "k8s-file:/some/path",
                should_error: false,
                expected: Driver::K8sFile("/some/path".into()).into(),
            },
            Tc {
                input: "journald:/some/path",
                should_error: false,
                expected: Driver::Journald.into(),
            },
            Tc {
                input: "journald",
                should_error: false,
                expected: Driver::Journald.into(),
            },
            Tc {
                input: "journald:",
                should_error: false,
                expected: Driver::Journald.into(),
            },
            Tc {
                input: ":/some/path",
                should_error: true,
                expected: None,
            },
            Tc {
                input: "wrong:/some/path",
                should_error: true,
                expected: None,
            },
            Tc {
                input: "none",
                should_error: false,
                expected: Driver::None.into(),
            },
            Tc {
                input: "off",
                should_error: false,
                expected: Driver::Off.into(),
            },
            Tc {
                input: "null",
                should_error: false,
                expected: Driver::Null.into(),
            },
        ];
        for tc in test_cases {
            let res = ContainerLogging::parse_log_path(tc.input);
            if tc.should_error {
                assert!(res.is_err())
            } else {
                assert_eq!(res?, tc.expected.context("no driver provided")?)
            }
        }
        Ok(())
    }
}
