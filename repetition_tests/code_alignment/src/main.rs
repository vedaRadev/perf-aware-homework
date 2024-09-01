use repetition_tester::{
    TimeTestResult,
    TimeTestSection,
    RepetitionTester,
};

#[link(name = "code_alignment_asm")]
extern "C" {
    fn loop_aligned_64(count: u64);
    fn loop_aligned_1(count: u64);
    fn loop_aligned_15(count: u64);
    fn loop_aligned_31(count: u64);
    fn loop_aligned_63(count: u64);
}

const LOOP_COUNT: u64 = 2u64.pow(24); // 16 mb

fn test_loop_aligned_64(_: &mut ()) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe { loop_aligned_64(LOOP_COUNT); }
    section.end(LOOP_COUNT)
}

fn test_loop_aligned_1(_: &mut ()) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe { loop_aligned_1(LOOP_COUNT); }
    section.end(LOOP_COUNT)
}

fn test_loop_aligned_15(_: &mut ()) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe { loop_aligned_15(LOOP_COUNT); }
    section.end(LOOP_COUNT)
}

fn test_loop_aligned_31(_: &mut ()) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe { loop_aligned_31(LOOP_COUNT); }
    section.end(LOOP_COUNT)
}

fn test_loop_aligned_63(_: &mut ()) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe { loop_aligned_63(LOOP_COUNT); }
    section.end(LOOP_COUNT)
}

fn main() {
    let mut repetition_tester = RepetitionTester::new(());
    repetition_tester.register_test(test_loop_aligned_64, "loop_aligned_64");
    repetition_tester.register_test(test_loop_aligned_1, "loop_aligned_1");
    repetition_tester.register_test(test_loop_aligned_15, "loop_aligned_15");
    repetition_tester.register_test(test_loop_aligned_31, "loop_aligned_31");
    repetition_tester.register_test(test_loop_aligned_63, "loop_aligned_63");

    repetition_tester.run_tests();
}
