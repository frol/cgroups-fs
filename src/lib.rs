//! Cgroup-fs is a minimal wrapper around Linux Control Groups (cgroups) filesystem (usually
//! mounted as `/sys/fs/cgroup`).
//!
//! # Examples
//!
//! ## Get memory usage from root cgroup
//!
//! ```
//! let root_cgroup = cgroups_fs::CgroupName::new("");
//! let root_memory_cgroup = cgroups_fs::Cgroup::new(&root_cgroup, "memory");
//! println!(
//!     "Current memory usage is {} bytes",
//!     root_memory_cgroup.get_value::<u64>("memory.usage_in_bytes").unwrap()
//! );
//! ```
//!
//! ## Measure memory usage of a child process
//!
//! Read [the `CgroupsCommandExt` documentation].
//!
//! [the `CgroupsCommandExt` documentation]: trait.CgroupsCommandExt.html#impl-CgroupsCommandExt
#![cfg(target_os = "linux")]
#![deny(missing_docs)]
#![deny(unsafe_code)]

use std::fs;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};

use nix;

/// A common structure holding a cgroups name (path).
#[derive(Debug)]
pub struct CgroupName {
    mount_point: PathBuf,
    name: PathBuf,
}

impl CgroupName {
    /// Defines a new cgroups name.
    ///
    /// Notes:
    /// * It does not create any cgroups. It is just an API abstraction layer. Learn more about
    /// [`Cgroup::new`], [`Cgroup::create`], [`Cgroup::remove`], and [`AutomanagedCgroup::init`]
    /// methods.
    ///
    /// [`Cgroup::new`]: struct.Cgroup.html#method.new
    /// [`Cgroup::create`]: struct.Cgroup.html#method.create
    /// [`Cgroup::remove`]: struct.Cgroup.html#method.remove
    /// [`AutomanagedCgroup::init`]: struct.AutomanagedCgroup.html#method.init
    pub fn new<P>(name: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            // TODO: auto-discover the cgroups FS mount-point
            mount_point: "/sys/fs/cgroup".into(),
            name: name.as_ref().to_path_buf(),
        }
    }
}

/// A controller of a specific cgroups namespace.
///
/// This type supports a number of operations for manipulating with a cgroups namespace.
#[derive(Debug)]
pub struct Cgroup {
    root: PathBuf,
}

impl Cgroup {
    /// Defines a cgroup relation.
    ///
    /// Notes:
    /// * It does not create any cgroups. It is just an API abstraction layer. Learn more about
    /// [`Cgroup::create`], [`Cgroup::remove`], and [`AutomanagedCgroup::init`] methods.
    ///
    /// [`Cgroup::create`]: #method.create
    /// [`Cgroup::remove`]: #method.remove
    /// [`AutomanagedCgroup::init`]: struct.AutomanagedCgroup.html#method.init
    pub fn new(cgroup_name: &CgroupName, subsystem: &str) -> Self {
        Self {
            root: cgroup_name
                .mount_point
                .join(subsystem)
                .join(&cgroup_name.name),
        }
    }

    /// Creates a cgroups namespace.
    ///
    /// Notes:
    /// * Keep in mind the usual filesystem permissions (owner, group, and mode bits).
    pub fn create(&self) -> io::Result<()> {
        fs::create_dir(&self.root).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "Cgroup cannot be created due to: {} (tried creating {:?} directory)",
                    error, self.root
                ),
            )
        })
    }

    /// Removes a cgroups namespace.
    ///
    /// Notes:
    /// * This method will fail if there are nested cgroups.
    /// * Keep in mind the usual filesystem permissions (owner, group, and mode bits).
    pub fn remove(&self) -> io::Result<()> {
        fs::remove_dir(&self.root).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "Cgroup cannot be removed due to: {} (tried removing {:?} directory)",
                    error, self.root
                ),
            )
        })
    }

    /// Sets a binary or string value to the cgroup control file.
    pub fn set_raw_value<V>(&self, key: &str, value: V) -> io::Result<()>
    where
        V: AsRef<[u8]>,
    {
        let key = self.root.join(key);
        fs::write(&key, value).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "Cgroup value under key {:?} cannot be set due to: {}",
                    key, error
                ),
            )
        })
    }

    /// Sets a value to the cgroup control file.
    pub fn set_value<V>(&self, key: &str, value: V) -> io::Result<()>
    where
        V: Copy + ToString,
    {
        self.set_raw_value(key, value.to_string())
    }

    /// Gets a string value from cgroup control file.
    pub fn get_raw_value(&self, key: &str) -> io::Result<String> {
        let key = self.root.join(key);
        fs::read_to_string(&key).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "Cgroup value under key {:?} cannot be read due to: {}",
                    key, error
                ),
            )
        })
    }

    /// Gets a value from cgroup control file.
    pub fn get_value<T>(&self, key: &str) -> io::Result<T>
    where
        T: std::str::FromStr,
    {
        self.get_raw_value(key)?
            .trim_end()
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "could not parse the value"))
    }

    fn tasks_absolute_path(&self) -> PathBuf {
        self.root.join("tasks")
    }

    /// Attaches a task (thread) to the cgroup.
    pub fn add_task(&self, pid: nix::unistd::Pid) -> io::Result<()> {
        fs::write(self.tasks_absolute_path(), pid.to_string()).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "A task cannot be added to cgroup {:?} due to: {}",
                    self.root, error
                ),
            )
        })
    }

    /// Lists tasks (threads) attached to the cgroup.
    pub fn get_tasks(&self) -> io::Result<Vec<nix::unistd::Pid>> {
        Ok(fs::read_to_string(self.tasks_absolute_path())
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "Tasks cannot be read from cgroup {:?} due to: {}",
                        self.root, error
                    ),
                )
            })?
            .split_whitespace()
            .map(|pid| nix::unistd::Pid::from_raw(pid.parse().unwrap()))
            .collect())
    }

    /// Sends a specified Unix Signal to all the tasks in the Cgroup.
    pub fn send_signal_to_all_tasks(&self, signal: nix::sys::signal::Signal) -> io::Result<usize> {
        let tasks = self.get_tasks()?;
        let tasks_count = tasks.len();
        for task in tasks {
            nix::sys::signal::kill(task, signal).ok();
        }
        Ok(tasks_count)
    }

    /// Kills (SIGKILL) all the attached to the cgroup tasks.
    ///
    /// WARNING: The naive implementation turned out to be not reliable enough for the fork-bomb
    /// use-case. To implement a reliable `kill_all` method, use `freezer` Cgroup. It is decided to
    /// move such extensions into a separate crate (to be announced).
    #[deprecated(
        since = "1.0.1",
        note = "please, use `freezer` cgroup to implement `kill_all_tasks` reliably (https://gitlab.com/dots.org.ua/ddots-runner/blob/d967ee3ba9de364dfb5a2e1a4f468586efb504f8/src/extensions/process.rs#L132-166)"
    )]
    pub fn kill_all_tasks(&self) -> io::Result<()> {
        for _ in 0..100 {
            if self.send_signal_to_all_tasks(nix::sys::signal::Signal::SIGKILL)? == 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(1));
        }
        Err(io::Error::new(
            io::ErrorKind::Other,
            "child subprocess(es) survived SIGKILL",
        ))
    }
}

/// An automatically managed controller of a specific cgroups subsystem.
///
/// It is a wrapper around [`Cgroup`] type which automatically creates (on [`init`]) and removes
/// (on [`drop`]) a cgroup in a given subsystem.
///
/// Since it is a wrapper, all the methods from [`Cgroup`] type are directly available for
/// `AutomanagedCgroup` instances.
///
/// [`Cgroup`]: struct.Cgroup.html
/// [`init`]: struct.AutomanagedCgroup.html#method.init
/// [`drop`]: struct.AutomanagedCgroup.html#impl-Drop
#[derive(Debug)]
pub struct AutomanagedCgroup {
    inner: Cgroup,
}

impl AutomanagedCgroup {
    /// Inits a cgroup, which means that it creates a cgroup in a given subsystem.
    ///
    /// Notes:
    /// * If there is an existing cgroup, it will be recreated from scratch. If that is not what
    ///   you what, consider using [`Cgroup`] type instead.
    /// * The cgroup will be automatically removed once the `AutomanagedCgroup` instance is
    ///   dropped.
    ///
    /// [`Cgroup`]: struct.Cgroup.html
    pub fn init(cgroup_name: &CgroupName, subsystem: &str) -> io::Result<Self> {
        let inner = Cgroup::new(cgroup_name, subsystem);
        if let Err(error) = inner.create() {
            match inner.get_tasks() {
                Err(_) => return Err(error),
                Ok(tasks) => {
                    if !tasks.is_empty() {
                        return Err(error);
                    }
                }
            }
            inner.remove().is_ok();
            inner.create()?;
        }
        Ok(Self { inner })
    }
}

impl std::ops::Deref for AutomanagedCgroup {
    type Target = Cgroup;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AsRef<Cgroup> for AutomanagedCgroup {
    fn as_ref(&self) -> &Cgroup {
        &self
    }
}

impl Drop for AutomanagedCgroup {
    fn drop(&mut self) {
        drop(self.inner.remove());
    }
}

/// This trait is designed to extend `std::process::Command` type with helpers for Cgroups.
pub trait CgroupsCommandExt {
    /// Specifies the Cgroups the executed process will be put into on start.
    fn cgroups(&mut self, cgroups: &[impl AsRef<Cgroup>]) -> &mut Self;
}

impl CgroupsCommandExt for std::process::Command {
    /// Specifies the Cgroups the executed process will be put into on start.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let my_cgroup = cgroups_fs::CgroupName::new("my-cgroup");
    /// let my_memory_cgroup = cgroups_fs::AutomanagedCgroup::init(&my_cgroup, "memory").unwrap();
    ///
    /// use cgroups_fs::CgroupsCommandExt;
    /// let output = std::process::Command::new("echo")
    ///     .arg("Hello world")
    ///     .cgroups(&[&my_memory_cgroup])
    ///     .output()
    ///     .expect("Failed to execute command");
    ///
    /// println!(
    ///     "The echo process used {} bytes of RAM.",
    ///     my_memory_cgroup.get_value::<u64>("memory.max_usage_in_bytes").unwrap()
    /// );
    /// ```
    fn cgroups(&mut self, cgroups: &[impl AsRef<Cgroup>]) -> &mut Self {
        let tasks_paths = cgroups
            .iter()
            .map(|cgroup| cgroup.as_ref().tasks_absolute_path())
            .collect::<Vec<PathBuf>>();
        self.before_exec(move || {
            let pid = std::process::id().to_string();
            for tasks_path in &tasks_paths {
                fs::write(tasks_path, &pid)?;
            }
            Ok(())
        })
    }
}
