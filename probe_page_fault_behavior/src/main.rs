mod pointer_decomposition;

use std::io::Write;
use performance_metrics::read_os_page_fault_count;
use pointer_decomposition::DecomposedPointer;
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

    let page_count: usize = args.next()
        .expect("required arg \"page count\" (# pages to allocate) not supplied")
        .parse()
        .expect("failed to parse usize from given \"page count\" arg");
    let mut write_direction: WriteDirection = WriteDirection::Forward;
    let mut output_file: Option<std::fs::File> = None;
    while let Some(option) = args.next() {
        match option.as_str() {
            "--write-direction" => write_direction = WriteDirection::from(args.next().expect("--write-direction expects 1 argument: \"forward\"/\"backward\"")),
            "--output-file" => output_file = args.next()
                .map_or_else(|| panic!("expected output filename"), std::fs::File::create)
                .map_or_else(|err| panic!("failed to create output file: {err}"), Some),

            _ => panic!("Unrecognized option \"{}\"", option),
        }
    }

    let total_bytes = PAGE_SIZE * page_count;
    if let Some(file) = &mut output_file {
        _ = writeln!(file, "Page Count, Pages Touched, Page Faults, Extra Faults");
    }

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
            let is_start_of_page = index % 4096 == 0;
            let mut pages_mapped_begin = 0;

            let addr_to_write = unsafe { buffer_start.add(index) };

            if is_start_of_page { pages_mapped_begin = read_os_page_fault_count(); }
            unsafe { *addr_to_write = index as u8; }
            if is_start_of_page {
                let pages_mapped = read_os_page_fault_count() - pages_mapped_begin;
                if pages_mapped == 0 { continue; }

                let page_number = index / 4096;
                println!(
                    "first write to page {:03} mapped {:02} pages: addr {:#0x} ({})",
                    page_number,
                    pages_mapped,
                    addr_to_write as u64,
                    DecomposedPointer::new(addr_to_write as u64)
                );
            }

        }
        let page_faults_end = read_os_page_fault_count();

        let page_faults = page_faults_end - page_faults_begin;
        let extra_faults = page_faults - pages_to_touch as u64;

        if let Some(file) = &mut output_file {
            _ = writeln!(file, "{page_count}, {pages_to_touch}, {page_faults}, {extra_faults}");
        } 

        println!("wrote to {pages_to_touch} pages, mapped {page_faults} pages leaving {extra_faults} unused\n");

        unsafe { VirtualFree(buffer_start as *mut c_void, 0, MEM_RELEASE); }
    }
}
