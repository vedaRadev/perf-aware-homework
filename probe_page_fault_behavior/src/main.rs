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

// TODO cleanup
// Find another way to record data for the csv so that we can just do one run through instead of
// being like "touch 0 pages, tough 1 page, touch 2 pages, touch 3 pages, etc."
// Will help reduce some of the hacky code I introduced to speed up output when not collecting data
// in a csv.
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

    // If the output file is provided, then monotonically increase the amount of pages we touch by
    // one each time up to the given page_count and generate that CSV, otherwise just skip straight
    // to writing all page_count pages to quickly get some relevant output on the screen.
    let range = if output_file.is_some() { 0 ..= page_count } else { page_count ..= page_count };
    for pages_to_touch in range {
        let buffer_start = unsafe { VirtualAlloc(std::ptr::null_mut(), total_bytes, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE) };
        let buffer_start = buffer_start as *mut u8;
        if buffer_start.is_null() {
            panic!("Failed to allocate memory");
        }
        println!(
            "start of alloc'd virtual range: {:#0x} ({})",
            buffer_start as u64,
            DecomposedPointer::new(buffer_start as u64)
        );

        let range = get_range(&write_direction, pages_to_touch);
        let mut prev_map_pattern = 0;
        let mut prev_addr: *const u8 = std::ptr::null();
        let page_faults_begin = read_os_page_fault_count();
        for page_number in range {
            // write one byte per page
            let addr_to_write = unsafe { buffer_start.add(page_number * PAGE_SIZE) };
            let pages_mapped_begin = read_os_page_fault_count();
            unsafe { *addr_to_write = page_number as u8; }
            let pages_mapped_end = read_os_page_fault_count();

            let pages_mapped = pages_mapped_end - pages_mapped_begin;
            if pages_to_touch == page_count
                && !prev_addr.is_null()
                && pages_mapped != prev_map_pattern
                // On windows, a 16 page prefetch seems to be common, so just filter out all the
                // noise of prefetching 16 pages then mapping none for a while (which is expected).
                // At least, this is expected when writing pages in a forward pattern; when going
                // backward Windows doesn't seem to ever prefetch...
                && !(pages_mapped == 16 && prev_map_pattern == 0 || pages_mapped == 0 && prev_map_pattern == 16)
            {
                println!("mapping pattern changed");
                println!(
                    "\tmapped this time: {:02} at addr {:#0x} ({})",
                    pages_mapped,
                    addr_to_write as u64,
                    DecomposedPointer::new(addr_to_write as u64),
                );
                println!(
                    "\tmapped last time: {:02} at addr {:#0x} ({})",
                    prev_map_pattern,
                    prev_addr as u64,
                    DecomposedPointer::new(prev_addr as u64)
                );

                prev_map_pattern = pages_mapped;
            }

            prev_addr = addr_to_write;
        }
        let page_faults_end = read_os_page_fault_count();

        let page_faults = page_faults_end - page_faults_begin;
        let extra_faults = page_faults - pages_to_touch as u64;

        if let Some(file) = &mut output_file {
            _ = writeln!(file, "{page_count}, {pages_to_touch}, {page_faults}, {extra_faults}");
        } 

        if pages_to_touch == page_count - 1 {
            println!("wrote to {pages_to_touch} pages, mapped {page_faults} pages leaving {extra_faults} unused\n");
        }

        unsafe { VirtualFree(buffer_start as *mut c_void, 0, MEM_RELEASE); }
    }
}
