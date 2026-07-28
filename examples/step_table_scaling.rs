//! Measure how STEP reading scales with entity count, holding everything else
//! fixed.
//!
//! Timings taken over real CAD files cannot answer this on their own. File size
//! is a poor proxy for entity count, the entity type mix varies between files,
//! and the largest models peak at several gigabytes, so on a loaded machine a
//! superlinear algorithm and ordinary paging produce the same curve. This
//! generates inputs that differ only in how many entities they contain and
//! keeps them small enough to stay resident, so the shape of the curve means
//! what it appears to mean.
//!
//! Run with:
//!
//! ```console
//! cargo run --release --example step_table_scaling
//! ```

use std::{fmt::Write as _, time::Instant};

use look::step::part21;
use truck_stepio::r#in::Table;

/// Entity counts to sweep. The top of the range has to stay small enough that
/// the syntax tree is comfortably resident, or the measurement turns into a
/// measurement of the page file.
const COUNTS: [usize; 6] = [125_000, 250_000, 500_000, 1_000_000, 1_500_000, 2_000_000];

fn main() {
    println!(
        "{:>10} {:>10} {:>12} {:>10} {:>12} {:>10} {:>10} {:>10}",
        "entities", "MB", "parse ms", "us/ent", "table ms", "us/ent", "peak MB", "freeMB"
    );

    for count in COUNTS {
        let text = generate(count);
        let megabytes = text.len() as f64 / 1.0e6;

        let started = Instant::now();
        let exchange = part21::parse(&text).expect("generated STEP should parse");
        let parse_ms = started.elapsed().as_secs_f64() * 1000.0;

        let section = exchange.data.first().expect("one data section");
        let started = Instant::now();
        let table = Table::from_data_section(section);
        let table_ms = started.elapsed().as_secs_f64() * 1000.0;

        // Keep the table alive across the measurement so nothing is optimised
        // away, and report a field to prove it was actually built.
        let points = table.cartesian_point.len();
        assert_eq!(points, count / ENTITIES_PER_POINT);

        println!(
            "{count:>10} {megabytes:>10.1} {parse_ms:>12.1} {:>10.3} {table_ms:>12.1} {:>10.3} \
             {:>10.0} {:>10.0}",
            parse_ms * 1000.0 / count as f64,
            table_ms * 1000.0 / count as f64,
            peak_working_set_bytes() as f64 / 1.0e6,
            available_bytes() as f64 / 1.0e6
        );
    }
}

/// Peak resident memory of this process, so the curve can be read against the
/// point where the working set stops fitting.
#[cfg(windows)]
fn peak_working_set_bytes() -> u64 {
    #[repr(C)]
    #[derive(Default)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool: usize,
        quota_paged_pool: usize,
        quota_peak_non_paged_pool: usize,
        quota_non_paged_pool: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    unsafe extern "C" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(
            process: isize,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }
    let mut counters = ProcessMemoryCounters {
        cb: size_of::<ProcessMemoryCounters>() as u32,
        ..Default::default()
    };
    unsafe {
        if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) == 0 {
            return 0;
        }
    }
    counters.peak_working_set_size as u64
}

/// Physical memory still available to the process.
#[cfg(windows)]
fn available_bytes() -> u64 {
    #[repr(C)]
    #[derive(Default)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_physical: u64,
        available_physical: u64,
        total_page_file: u64,
        available_page_file: u64,
        total_virtual: u64,
        available_virtual: u64,
        available_extended_virtual: u64,
    }
    unsafe extern "C" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }
    let mut status = MemoryStatusEx {
        length: size_of::<MemoryStatusEx>() as u32,
        ..Default::default()
    };
    unsafe {
        if GlobalMemoryStatusEx(&mut status) == 0 {
            return 0;
        }
    }
    status.available_physical
}

#[cfg(not(windows))]
fn peak_working_set_bytes() -> u64 {
    0
}

#[cfg(not(windows))]
fn available_bytes() -> u64 {
    0
}

/// How many entities each generated point contributes: the point itself plus
/// the direction and placement that reference it, so the graph has references
/// to resolve rather than being a flat list of leaves.
const ENTITIES_PER_POINT: usize = 3;

/// Build a syntactically valid exchange structure with a fixed type mix and
/// the requested number of entities.
fn generate(count: usize) -> String {
    let groups = count / ENTITIES_PER_POINT;
    let mut text = String::with_capacity(groups * 160);
    text.push_str(
        "ISO-10303-21;\n\
         HEADER;\n\
         FILE_DESCRIPTION(('scaling'),'2;1');\n\
         FILE_NAME('scaling','2026-01-01T00:00:00',(''),(''),'','','');\n\
         FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\n\
         ENDSEC;\n\
         DATA;\n",
    );
    for group in 0..groups {
        let point = group * 3 + 1;
        let direction = point + 1;
        let placement = point + 2;
        let value = group as f64 * 0.001;
        let _ = writeln!(
            text,
            "#{point} = CARTESIAN_POINT('',({value:.6},{value:.6},{value:.6}));\n\
             #{direction} = DIRECTION('',(0.,0.,1.));\n\
             #{placement} = AXIS2_PLACEMENT_3D('',#{point},#{direction},$);"
        );
    }
    text.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    text
}
