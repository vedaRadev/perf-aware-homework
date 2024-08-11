use performance_metrics::read_os_page_fault_count;
use winapi::{
    ctypes::c_void,
    um::{
        winnt::{ MEM_RESERVE, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE },
        memoryapi::{ VirtualAlloc, VirtualFree },
    }
};

/// The testing page size. May not be the OS page size.
const PAGE_SIZE: usize = 4096;

enum WriteDirection {
    Forward,
    Backward,
}

impl std::convert::From<String> for WriteDirection {
    fn from(s: String) -> Self {
        match s.to_uppercase().as_str() {
            "FORWARD" => WriteDirection::Forward,
            "BACKWARD" => WriteDirection::Backward,
            _ => panic!("invalid write direction: \"{}\"", s)
        }
    }
}

fn get_range(write_direction: &WriteDirection, size: usize) -> Box<dyn Iterator<Item = usize>> {
    match write_direction {
        WriteDirection::Forward => Box::new(0 .. size),
        WriteDirection::Backward => Box::new((0 .. size).rev()),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let page_count = args.next()
        .expect("required arg \"page count\" (# pages to allocate) not supplied")
        .parse()
        .expect("failed to parse u64 from given \"page count\" arg");
    let write_direction = args.next().map_or(WriteDirection::Forward, WriteDirection::from);

    let total_bytes = PAGE_SIZE * page_count;
    println!("Page Count, Pages Touched, Page Faults, Extra Faults");

    for pages_to_touch in 0 .. page_count {
        let buffer_start = unsafe { VirtualAlloc(std::ptr::null_mut(), total_bytes, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE) };
        let buffer_start = buffer_start as *mut u8;
        if buffer_start.is_null() {
            panic!("Failed to allocate memory");
        }

        let bytes_to_touch = PAGE_SIZE * pages_to_touch;
        let range = get_range(&write_direction, bytes_to_touch);
        let page_faults_begin = read_os_page_fault_count();
        for index in range {
            unsafe { *buffer_start.add(index) = index as u8; }
        }
        let page_faults_end = read_os_page_fault_count();

        let page_faults = page_faults_end - page_faults_begin;
        let extra_faults = page_faults - pages_to_touch as u64;
        println!("{page_count}, {pages_to_touch}, {page_faults}, {extra_faults}");

        unsafe { VirtualFree(buffer_start as *mut c_void, 0, MEM_RELEASE); }
    }
}
