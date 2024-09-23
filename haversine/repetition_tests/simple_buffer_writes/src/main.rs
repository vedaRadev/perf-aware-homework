use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
};
use std::{
    slice,
    alloc::{
        alloc,
        dealloc,
        Layout
    }
};

// These tests highlight windows' page fault behavior.
// Writing bytes forward through unmapped virtual address space will be FASTER than writing it
// backward through unmapped virtual address space.
//
// Windows displays 16-page premapping after the first 16 maps into a new page table (or after 16
// maps into the page table in which the virtual address you receive is). It only does this when
// writing FORWARD through pages, but not when writing backward through pages in a table.
//
// Note here that windows will report these two tests as generating the same amount of "page
// faults." Windows is actually lying to us because the "page faults" it report is actually "number
// of pages mapped" when a page fault is generated.

const BUFFER_SIZE: usize = 2usize.pow(26); // 16mb

#[inline(never)]
#[no_mangle]
fn write_all_bytes(_: &mut ()) -> TimeTestResult {
    let layout = Layout::array::<u8>(BUFFER_SIZE).expect("Failed to create memory layout");
    let buffer_start = unsafe { alloc(layout) };
    let buffer = unsafe { slice::from_raw_parts_mut(buffer_start, BUFFER_SIZE) };

    let test_section = TimeTestSection::begin();
    for (index, element) in buffer.iter_mut().enumerate() {
        *element = index as u8;
    }
    let result = test_section.end(BUFFER_SIZE as u64);

    unsafe { dealloc(buffer_start, layout); }

    result
}

#[allow(dead_code)]
#[inline(never)]
fn write_all_bytes_backward(_: &mut ()) -> TimeTestResult {
    let layout = Layout::array::<u8>(BUFFER_SIZE).expect("Failed to create memory layout");
    let buffer_start = unsafe { alloc(layout) };
    let buffer = unsafe { slice::from_raw_parts_mut(buffer_start, BUFFER_SIZE) };

    let test_section = TimeTestSection::begin();
    for (index, element) in buffer.iter_mut().enumerate().rev() {
        *element = index as u8;
    }
    let result = test_section.end(BUFFER_SIZE as u64);

    unsafe { dealloc(buffer_start, layout); }

    result
}

fn main() {
    let mut repetition_tester = RepetitionTester::new(());
    repetition_tester.register_test(write_all_bytes, "write all bytes forward");
    repetition_tester.register_test(write_all_bytes_backward, "write all bytes backward");
    repetition_tester.run_tests();
}
