//! Performance and memory of the engine: nanoseconds per unit of fuel and peak
//! heap memory for normal and adversarial workloads, on language versions 1 and 2.
//!
//! `cargo test -p tccl --release --test perf -- --ignored --nocapture`
//!
//! TCCL is a tree-walking interpreter written in Rust. Rust gives memory safety
//! and predictable performance, not native speed for contracts: what matters for
//! the network is that fuel tracks CPU time, so a full block stays fast.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tccl::program::Value;
use tccl::sim::{account, DeployOptions, Simulator};

struct Counting;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let now = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(now, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const WORKLOADS: &[(&str, &str, i128)] = &[
    ("arithmetic loop", "contract B\naction run(n: int) -> int:\n    let s: int = 0\n    for i in range(0, n):\n        s += i * 3 % 7\n    return s\n", 200_000),
    ("storage map writes", "contract B\nstate m: map[int, int]\naction run(n: int) -> int:\n    for i in range(0, n):\n        m[i] = i + 1\n    return n\n", 3_000),
    ("blake3 chain", "contract B\naction run(n: int) -> int:\n    let h: bytes = 0x00\n    for i in range(0, n):\n        h = blake3(h + to_bytes(i))\n    return len(h)\n", 20_000),
    ("ed25519 (invalid sigs)", "contract B\naction run(n: int) -> int:\n    let c: int = 0\n    for i in range(0, n):\n        if verify_ed25519(0x3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c, to_bytes(i), 0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000):\n            c += 1\n    return c\n", 500),
    ("records build", "contract B\nrecord P:\n    a: int\n    b: text\naction run(n: int) -> int:\n    let c: int = 0\n    for i in range(0, n):\n        let p: P = P(a: i, b: \"item\")\n        c += p.a\n    return c\n", 100_000),
    ("ADVERSARIAL copy list[text]", "contract B\naction run(n: int) -> int:\n    let xs: list[text] = []\n    for i in range(0, 4000):\n        xs.push(\"a\")\n    let c: int = 0\n    for i in range(0, n):\n        c += len(xs)\n    return c\n", 2_000),
    ("ADVERSARIAL decode stored list", "contract B\nstate m: map[int, list[text]]\naction run(n: int) -> int:\n    let xs: list[text] = []\n    for i in range(0, 4000):\n        xs.push(\"a\")\n    m[1] = xs\n    let c: int = 0\n    for i in range(0, n):\n        c += len(m[1])\n    return c\n", 2_000),
    ("ADVERSARIAL big local lists", "contract B\naction run(n: int) -> int:\n    let big: text = \"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\"\n    for i in range(0, 9):\n        big += big\n    let xs: list[text] = []\n    for i in range(0, n):\n        xs.push(big)\n    return len(xs)\n", 1_000),
];

#[test]
#[ignore]
fn fuel_time_and_memory() {
    println!("{:<34} {:>4} {:>11} {:>10} {:>10} {:>11} {:>12}", "workload", "lang", "fuel", "ms", "ns/fuel", "peak heap", "50M block s");
    for (name, src, n) in WORKLOADS {
        for language in [1u16, 2] {
            if language == 1 && src.contains("record") {
                continue;
            }
            let mut sim = Simulator::new();
            let (c, d) = sim.deploy_with(src, account("bench"), vec![], 0, &DeployOptions { language, final_code: true }).unwrap();
            d.result.unwrap();
            let mut best = f64::MAX;
            let (mut fuel, mut peak, mut outcome) = (0, 0, String::new());
            for _ in 0..3 {
                let base = CURRENT.load(Ordering::Relaxed);
                PEAK.store(base, Ordering::Relaxed);
                let t = Instant::now();
                let r = sim.call(&c, account("bench"), "run", vec![Value::Int(*n)], 0).unwrap();
                let ns = t.elapsed().as_nanos() as f64;
                peak = PEAK.load(Ordering::Relaxed) - base;
                fuel = r.fuel_used;
                outcome = match r.result {
                    Ok(_) => String::new(),
                    Err(e) => format!(" ({e})"),
                };
                best = best.min(ns);
            }
            let per = best / fuel.max(1) as f64;
            println!(
                "{name:<34} {language:>4} {fuel:>11} {:>10.2} {per:>10.1} {:>8.1} MB {:>12.1}{outcome}",
                best / 1e6,
                peak as f64 / 1e6,
                per * 50e6 / 1e9
            );
        }
    }
}
