use crate::util::{encoding, prelude::*};
use chrono::prelude::*;
use parking_lot::Mutex as SyncMutex;
use serde::Serialize;
use std::fmt;
use std::time::Duration;
use sysinfo::{CpuExt, SystemExt};

pub(crate) struct SysInfoService {
    sysinfo: SyncMutex<sysinfo::System>,
}

impl SysInfoService {
    fn refresh_kind() -> sysinfo::RefreshKind {
        sysinfo::RefreshKind::new()
            .with_cpu(sysinfo::CpuRefreshKind::everything())
            .with_memory()
    }

    pub(crate) fn new() -> Self {
        Self {
            sysinfo: sysinfo::System::new_with_specifics(Self::refresh_kind()).into(),
        }
    }

    pub(crate) fn to_human_readable(&self) -> String {
        let mut inf = self.sysinfo.lock();
        inf.refresh_specifics(Self::refresh_kind());

        let inf = inf;
        let mem = humansize::make_format(humansize::DECIMAL);
        let percent = |x: &dyn fmt::Display| format!("{x:.1}%");

        let load_average = inf.load_average();
        let load_average = LoadAvg {
            one_min: percent(&load_average.one),
            five_min: percent(&load_average.five),
            fifteen_min: percent(&load_average.fifteen),
        };

        let cpu = inf.global_cpu_info();
        let cpu = Cpu {
            cores: inf.physical_core_count(),
            brand: cpu.brand().to_owned(),
            usage: percent(&cpu.cpu_usage()),
            freq: format!("{} MHz", cpu.frequency()),
        };

        let boot_time = Utc
            .timestamp(inf.boot_time().try_into().unwrap_or_default(), 0)
            .to_ymd_hms();

        let uptime = crate::util::human_duration(Duration::from_secs(inf.uptime()));

        let ram = Ram {
            available: mem(inf.available_memory()),
            base: Mem {
                used: mem(inf.used_memory()),
                free: mem(inf.free_memory()),
                total: mem(inf.total_memory()),
            },
        };
        let swap = Mem {
            used: mem(inf.used_swap()),
            free: mem(inf.free_swap()),
            total: mem(inf.total_swap()),
        };

        let info = SysInfo {
            platform: Platform {
                hostname: inf.host_name(),
                kernel: inf.kernel_version(),
                osname: inf.long_os_version(),
            },
            cpu,
            load_average,
            uptime: Uptime {
                total: uptime,
                booted: boot_time,
            },
            ram,
            swap,
        };

        encoding::to_yaml_string(&info)
    }
}

/// We define the structs with `serde(Serialize)` instead of using `json!()` macro
/// for such a simple task, because we want to preserve the order of the fields.
/// Enabling `preserve_order` feature for `serde_json` is not an option, because
/// it is a global feature and we don't want to enable it for the whole project.
///
/// The order the fields are defined in structs is easier to read for humans.
#[derive(Serialize)]
struct SysInfo {
    ram: Ram,
    swap: Mem,
    cpu: Cpu,
    load_average: LoadAvg,
    uptime: Uptime,
    platform: Platform,
}

#[derive(Serialize)]
struct Uptime {
    total: String,
    booted: String,
}

#[derive(Serialize)]
struct Platform {
    kernel: Option<String>,
    osname: Option<String>,
    hostname: Option<String>,
}

#[derive(Serialize)]
struct Cpu {
    usage: String,
    freq: String,
    cores: Option<usize>,
    brand: String,
}

#[derive(Serialize)]
struct LoadAvg {
    r#one_min: String,
    five_min: String,
    fifteen_min: String,
}

#[derive(Serialize)]
struct Ram {
    available: String,
    #[serde(flatten)]
    base: Mem,
}

#[derive(Serialize)]
struct Mem {
    used: String,
    free: String,
    total: String,
}
