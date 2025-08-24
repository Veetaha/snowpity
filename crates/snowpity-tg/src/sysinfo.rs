use crate::prelude::*;
use crate::util::display::{human_duration, human_size};
use crate::util::encoding;
use chrono::prelude::*;
use parking_lot::Mutex as SyncMutex;
use serde::Serialize;
use std::fmt;
use std::time::Duration;

pub(crate) struct SysInfoService {
    sysinfo: SyncMutex<sysinfo::System>,
}

impl SysInfoService {
    fn refresh_kind() -> sysinfo::RefreshKind {
        sysinfo::RefreshKind::nothing()
            .with_cpu(sysinfo::CpuRefreshKind::everything())
            .with_memory(sysinfo::MemoryRefreshKind::everything())
    }

    pub(crate) fn new() -> Self {
        Self {
            sysinfo: sysinfo::System::new_with_specifics(Self::refresh_kind()).into(),
        }
    }

    pub(crate) fn to_human_readable(&self) -> String {
        let mut info = self.sysinfo.lock();
        info.refresh_specifics(Self::refresh_kind());

        let info = info;
        let percent = |x: &dyn fmt::Display| format!("{x:.1}%");

        let load_average = sysinfo::System::load_average();
        let load_average = LoadAvg {
            one_min: percent(&load_average.one),
            five_min: percent(&load_average.five),
            fifteen_min: percent(&load_average.fifteen),
        };

        let cpu = Cpu {
            arch: sysinfo::System::cpu_arch(),
            usage: percent(&info.global_cpu_usage()),
        };

        let boot_time = Utc
            .timestamp_opt(
                sysinfo::System::boot_time().try_into().unwrap_or_default(),
                0,
            )
            .unwrap()
            .to_human_readable();

        let uptime = human_duration(Duration::from_secs(sysinfo::System::uptime()));

        let ram = Ram {
            available: human_size(info.available_memory()),
            base: Mem {
                used: human_size(info.used_memory()),
                free: human_size(info.free_memory()),
                total: human_size(info.total_memory()),
            },
        };
        let swap = Mem {
            used: human_size(info.used_swap()),
            free: human_size(info.free_swap()),
            total: human_size(info.total_swap()),
        };

        let info = SysInfo {
            platform: Platform {
                hostname: sysinfo::System::host_name(),
                kernel: sysinfo::System::kernel_version(),
                osname: sysinfo::System::long_os_version(),
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
    arch: String,
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
