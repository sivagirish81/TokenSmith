use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};

#[cfg(unix)]
use std::process::Command;

pub fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        use sysinfo::{Pid, ProcessesToUpdate, System};
        let mut sys = System::new_all();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        sys.process(Pid::from_u32(pid)).is_some()
    }
}

pub fn terminate_pid(pid: u32, force_after: Duration) -> Result<()> {
    #[cfg(unix)]
    {
        let term = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()?;
        if !term.success() {
            return Err(anyhow!("failed to send SIGTERM to {pid}"));
        }

        let start = Instant::now();
        while start.elapsed() < force_after {
            if !is_pid_alive(pid) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        if is_pid_alive(pid) {
            let kill = Command::new("kill")
                .arg("-KILL")
                .arg(pid.to_string())
                .status()?;
            if !kill.success() {
                return Err(anyhow!("failed to SIGKILL pid {pid}"));
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    {
        let status = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()?;
        if !status.success() {
            return Err(anyhow!("taskkill failed for pid {pid}"));
        }
        Ok(())
    }
}
