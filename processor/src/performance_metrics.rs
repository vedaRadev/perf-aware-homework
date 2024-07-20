extern crate profiling_proc_macros;
pub use profiling_proc_macros::profile;

use winapi::um::profileapi;
use std::mem;
use core::arch::x86_64::_rdtsc;
use profiling_proc_macros::__get_max_profile_sections;

struct ProfileSection {
    tsc_begin: u64,
    cycles_elapsed: u64,
    hits: u64,
    label: &'static str,
}

impl ProfileSection {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            tsc_begin: 0,
            cycles_elapsed: 0,
            hits: 0,
        }
    }
}

/// If you use any __GlobalProfiler methods directly instead of fielding its use through the
/// provided proc macros, I will find you and burn your home down.
pub struct __GlobalProfiler {
    /// The total elapsed cycles across all profile sections
    global_cycles: u64,
    sections: [Option<ProfileSection>; __get_max_profile_sections!()],
}

impl __GlobalProfiler {
    const fn init() -> Self {
        Self {
            global_cycles: 0,
            sections: [ const { None }; __get_max_profile_sections!() ],
        }
    }

    pub fn begin_section_profile(&mut self, label: &'static str, section_index: usize) {
        // // CRITICAL
        // // This should never happen because users should not be calling global profiler methods
        // // directly. This limit should be enforced by the profile! proc macro at compile time.
        // if section_index >= __get_max_profile_sections!() {
        //     panic!(
        //         "max profile sections limit reached or attempted to index beyond sections array: {}",
        //         section_index
        //     );
        // }

        let section = match &mut self.sections[section_index] {
            Some(section) => section,
            None => {
                let section = ProfileSection::new(label);
                self.sections[section_index] = Some(section);
                self.sections[section_index].as_mut().unwrap()
            }
        };

        section.hits += 1;
        section.tsc_begin = read_cpu_timer();
    }

    pub fn end_section_profile(&mut self, section_index: usize) {
        let tsc = read_cpu_timer();

        let section = self.sections[section_index].as_mut()
            .expect("No profile sections initialized. Was begin_profile_section called prior?");
        let cycles = tsc - section.tsc_begin;
        section.cycles_elapsed += cycles;
        self.global_cycles += cycles;
    }

    pub fn print_profile_info(&self, cpu_frequency_sample_millis: u64) {
        let cpu_frequency = get_cpu_frequency_estimate(cpu_frequency_sample_millis);
        println!(
            "Total time profiled: {:.2} ms (cpu freq estimate: {})",
            self.global_cycles as f64 / cpu_frequency as f64 * 1000.0,
            cpu_frequency,
        );

        for section in &self.sections {
            if section.is_none() { break; }
            let section = section.as_ref().unwrap();

            println!(
                "{}: {} hits, {} cycles ({:.3}%)",
                section.label,
                section.hits,
                section.cycles_elapsed,
                section.cycles_elapsed as f64 / self.global_cycles as f64 * 100.0
            );
        }
    }
}

/// DO NOT TOUCH THE GLOBAL PROFILER. USE THE PROVIDED PROC MACROS.
pub static mut __GLOBAL_PROFILER: __GlobalProfiler = __GlobalProfiler::init();

macro_rules! print_profile_info {
    ($cpu_frequency_sample_millis:expr) => {
        unsafe {
            $crate::performance_metrics::__GLOBAL_PROFILER.print_profile_info($cpu_frequency_sample_millis);
        }
    }
}

pub(crate) use print_profile_info;

fn get_os_timer_frequency() -> u64 {
    unsafe {
        let mut freq = mem::zeroed();
        profileapi::QueryPerformanceFrequency(&mut freq);
        *freq.QuadPart() as u64
    }
}

fn read_os_timer() -> u64 {
    unsafe {
        let mut counter = mem::zeroed();
        profileapi::QueryPerformanceCounter(&mut counter);
        *counter.QuadPart() as u64
    }
}

#[inline(always)]
fn read_cpu_timer() -> u64 { unsafe { _rdtsc() } }

/// Given a sample interval in milliseconds, returns an estimate of how many CPU timer ticks occur
/// in that interval.
fn get_cpu_frequency_estimate(ms_to_wait: u64) -> u64 {
    let os_timer_frequency = get_os_timer_frequency();
    let os_wait_time = os_timer_frequency * ms_to_wait / 1000;

    let mut os_elapsed: u64 = 0;
    let cpu_start = read_cpu_timer();
    let os_start = read_os_timer();
    while os_elapsed < os_wait_time {
        os_elapsed = read_os_timer() - os_start;
    }
    let cpu_end = read_cpu_timer();
    let cpu_elapsed = cpu_end - cpu_start;

    if os_elapsed > 0 {
        os_timer_frequency * cpu_elapsed / os_elapsed
    } else {
        0
    }
}
