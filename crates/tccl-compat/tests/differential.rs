//! Differential tests against the reference engine deployed on The Coin
//! (github.com/LucasBolla94/thecoin, crates/tccl at commit 8f620ea).
//!
//! * The frozen version 1 compiler must produce **byte-identical programs** and
//!   **identical errors** (message and position) for every input.
//! * Version 1 programs must run with identical results, fuel, storage and events.
//! * Version 1 sources compiled as version 2 must run with identical results and fuel.

use std::collections::BTreeMap;

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tccl/examples");

macro_rules! mem_host {
    ($name:ident, $krate:ident) => {
        #[derive(Clone, Default)]
        struct $name {
            storage: BTreeMap<Vec<u8>, Vec<u8>>,
            balance: u64,
            sent: Vec<([u8; 20], u64)>,
            events: Vec<(String, Vec<u8>)>,
            destroyed: bool,
        }
        impl $krate::vm::Host for $name {
            fn storage_read(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, $krate::VmError> {
                Ok(self.storage.get(key).cloned())
            }
            fn storage_write(&mut self, key: &[u8], value: Option<Vec<u8>>) -> Result<(), $krate::VmError> {
                match value {
                    Some(v) => self.storage.insert(key.to_vec(), v),
                    None => self.storage.remove(key),
                };
                Ok(())
            }
            fn balance(&mut self) -> Result<u64, $krate::VmError> {
                Ok(self.balance)
            }
            fn send(&mut self, to: &[u8; 20], amount: u64) -> Result<(), $krate::VmError> {
                if self.balance < amount {
                    return Err($krate::VmError::InsufficientBalance);
                }
                self.balance -= amount;
                self.sent.push((*to, amount));
                Ok(())
            }
            fn emit(&mut self, event: &str, fields: Vec<(String, $krate::Value)>) -> Result<(), $krate::VmError> {
                self.events.push((event.to_string(), borsh::to_vec(&fields).unwrap()));
                Ok(())
            }
            fn storage_items(&mut self) -> Result<u64, $krate::VmError> {
                Ok(self.storage.len() as u64)
            }
            fn destroy(&mut self, _to: &[u8; 20]) -> Result<(), $krate::VmError> {
                self.destroyed = true;
                Ok(())
            }
        }
    };
}

mem_host!(NewHost, tccl);
mem_host!(RefHost, tccl_v1);

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn examples() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(EXAMPLES)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "tccl"))
        .map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), std::fs::read_to_string(&p).unwrap()))
        .collect();
    out.sort();
    out
}

fn new_opts(version: u16) -> tccl::CompileOptions {
    tccl::CompileOptions { version, ..Default::default() }
}

fn compare_compile(src: &str) -> bool {
    let reference = tccl_v1::compile(src, &tccl_v1::CompileOptions::default());
    let ours = tccl::compile(src, &new_opts(1));
    match (&reference, &ours) {
        (Ok(a), Ok(b)) => assert_eq!(a.to_bytes(), b.to_bytes(), "compiled bytes differ for:\n{src}"),
        (Err(a), Err(b)) => {
            assert_eq!(a.message, b.message, "error differs for:\n{src}");
            assert_eq!((a.pos.line, a.pos.col), (b.pos.line, b.pos.col), "error position differs for:\n{src}");
        }
        _ => panic!("one compiler accepted and the other rejected:\n{src}\nreference: {:?}\nours: {:?}", reference.as_ref().err(), ours.as_ref().err()),
    }
    reference.is_ok()
}

#[test]
fn version_one_compiler_is_byte_identical() {
    let words = "contract const state event init action view fn payable let if elif else while for in break continue return require send emit destroy pass true false and or not ( ) [ ] : , . -> = += -= *= == != < <= > >= + - * / % \n \n    \n        x y n m int bool text bytes address list map caller value balance height self TCN len sha256 to_bytes to_text min max abs slice 0 1 -1 99999999999999999999999999999999999999999 0xab 0xabc \"s\" \"\\n\" # range push pop has remove".split(' ').collect::<Vec<_>>();
    let mut rng = Rng(0x2545f4914f6cdd1d);
    let mut accepted = 0;
    let mut total = 0;
    for (_, src) in examples() {
        total += 1;
        accepted += compare_compile(&src) as usize;
        // Mutations of real contracts exercise deep paths of the checker.
        for _ in 0..400 {
            let mut s: Vec<char> = src.chars().collect();
            for _ in 0..(1 + rng.below(3)) {
                if s.is_empty() {
                    break;
                }
                let i = rng.below(s.len());
                let j = (i + rng.below(12)).min(s.len());
                match rng.below(4) {
                    0 => {
                        s.drain(i..j);
                    }
                    1 => {
                        let piece: Vec<char> = s[i..j].to_vec();
                        s.splice(i..i, piece);
                    }
                    2 => {
                        let w = words[rng.below(words.len())];
                        s.splice(i..j, w.chars());
                    }
                    _ => {
                        let k = rng.below(s.len());
                        s.swap(i, k);
                    }
                }
            }
            let m: String = s.into_iter().collect();
            total += 1;
            accepted += compare_compile(&m) as usize;
        }
    }
    for _ in 0..3000 {
        let n = rng.below(60);
        let s = format!("contract R\n{}", (0..n).map(|_| words[rng.below(words.len())]).collect::<Vec<_>>().join(" "));
        total += 1;
        accepted += compare_compile(&s) as usize;
    }
    eprintln!("compared {total} sources ({accepted} accepted by both compilers)");
    assert!(accepted > 11, "the corpus must include valid programs");
}

fn arg_for(rng: &mut Rng, t: &tccl::Type, depth: usize) -> tccl::Value {
    use tccl::{Type, Value};
    match t {
        Type::Int => Value::Int([0, 1, 2, 5, 7, 100, 101, 1_000_000, -1, 300_000_000][rng.below(10)]),
        Type::Bool => Value::Bool(rng.below(2) == 0),
        Type::Text => Value::Text(["", "a", "hello", "item-1", "x".repeat(70).as_str()][rng.below(5)].to_string()),
        Type::Bytes => Value::Bytes(match rng.below(4) {
            0 => vec![],
            1 => vec![1, 2, 3],
            2 => vec![9; 32],
            _ => vec![7; 64],
        }),
        Type::Address => Value::Address([[1u8; 20], [2u8; 20], [3u8; 20], [0u8; 20]][rng.below(4)]),
        Type::List(inner) if depth < 3 => Value::List((0..rng.below(4)).map(|_| arg_for(rng, inner, depth + 1)).collect()),
        _ => Value::List(vec![]),
    }
}

fn to_ref(v: &tccl::Value) -> tccl_v1::Value {
    borsh::from_slice(&borsh::to_vec(v).unwrap()).unwrap()
}

#[test]
fn version_one_execution_is_identical() {
    let callers = [[1u8; 20], [2u8; 20], [3u8; 20]];
    let fuel = 2_000_000;
    let mut steps = 0;
    for (name, src) in examples() {
        let Ok(reference) = tccl_v1::compile(&src, &tccl_v1::CompileOptions::default()) else { continue };
        let ours_v1 = tccl::compile(&src, &new_opts(1)).unwrap();
        let ours_v2 = tccl::compile(&src, &new_opts(2)).unwrap();
        for seed in 1..=6u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9e3779b97f4a7c15) ^ name.len() as u64);
            let (mut h_ref, mut h_v1, mut h_v2) = (RefHost::default(), NewHost::default(), NewHost::default());
            let mut height = 1u64;
            let ctx_ref = |caller: [u8; 20], value: u64, height: u64| tccl_v1::vm::CallContext { caller, value, height, self_address: [9u8; 20] };
            let ctx_new = |caller: [u8; 20], value: u64, height: u64| tccl::vm::CallContext { caller, value, height, self_address: [9u8; 20] };
            // Deploy with generated init arguments.
            let init_args: Vec<tccl::Value> = ours_v1.find("init").map(|(_, f)| f.params.iter().map(|(_, t)| arg_for(&mut rng, t, 0)).collect()).unwrap_or_default();
            let value = [0u64, 0, 500_000_000][rng.below(3)];
            let caller = callers[rng.below(3)];
            for h in [&mut h_ref.balance, &mut h_v1.balance, &mut h_v2.balance] {
                *h = value;
            }
            let a = tccl_v1::vm::execute(&reference, tccl_v1::vm::Mode::Deploy, "init", init_args.iter().map(to_ref).collect(), &ctx_ref(caller, value, height), &mut h_ref, fuel);
            let b = tccl::vm::execute(&ours_v1, tccl::vm::Mode::Deploy, "init", init_args.clone(), &ctx_new(caller, value, height), &mut h_v1, fuel);
            let c = tccl::vm::execute(&ours_v2, tccl::vm::Mode::Deploy, "init", init_args.clone(), &ctx_new(caller, value, height), &mut h_v2, fuel);
            compare(&name, "deploy", &a, &b, &c, &h_ref, &h_v1, &h_v2);
            let entry: Vec<_> = ours_v1.abi().into_iter().filter(|f| f.name != "init").collect();
            for _ in 0..60 {
                steps += 1;
                let f = &entry[rng.below(entry.len())];
                let (_, def) = ours_v1.find(&f.name).unwrap();
                let args: Vec<tccl::Value> = def.params.iter().map(|(_, t)| arg_for(&mut rng, t, 0)).collect();
                let is_view = f.kind == tccl::program::FnKind::View;
                let value = if f.payable { [0u64, 1, 100_000_000, 500_000_000][rng.below(4)] } else { [0u64, 0, 0, 1][rng.below(4)] };
                let caller = callers[rng.below(3)];
                height += rng.below(40) as u64;
                let (sref, sv1, sv2) = (h_ref.clone(), h_v1.clone(), h_v2.clone());
                for h in [&mut h_ref.balance, &mut h_v1.balance, &mut h_v2.balance] {
                    *h += value;
                }
                let (mref, mnew) = if is_view { (tccl_v1::vm::Mode::View, tccl::vm::Mode::View) } else { (tccl_v1::vm::Mode::Action, tccl::vm::Mode::Action) };
                let a = tccl_v1::vm::execute(&reference, mref, &f.name, args.iter().map(to_ref).collect(), &ctx_ref(caller, value, height), &mut h_ref, fuel);
                let b = tccl::vm::execute(&ours_v1, mnew, &f.name, args.clone(), &ctx_new(caller, value, height), &mut h_v1, fuel);
                let c = tccl::vm::execute(&ours_v2, mnew, &f.name, args.clone(), &ctx_new(caller, value, height), &mut h_v2, fuel);
                compare(&name, &f.name, &a, &b, &c, &h_ref, &h_v1, &h_v2);
                if a.result.is_err() || is_view {
                    (h_ref, h_v1, h_v2) = (sref, sv1, sv2);
                }
            }
        }
    }
    eprintln!("compared {steps} calls on three engines");
}

#[allow(clippy::too_many_arguments)]
fn compare(name: &str, step: &str, a: &tccl_v1::vm::Outcome, b: &tccl::vm::Outcome, c: &tccl::vm::Outcome, ha: &RefHost, hb: &NewHost, hc: &NewHost) {
    let ra = a.result.as_ref().map(|v| borsh::to_vec(v).unwrap()).map_err(|e| e.to_string());
    let rb = b.result.as_ref().map(|v| borsh::to_vec(v).unwrap()).map_err(|e| e.to_string());
    let rc = c.result.as_ref().map(|v| borsh::to_vec(v).unwrap()).map_err(|e| e.to_string());
    assert_eq!(ra, rb, "{name} {step}: result (frozen v1)");
    assert_eq!(a.fuel_used, b.fuel_used, "{name} {step}: fuel (frozen v1)");
    assert_eq!(ra, rc, "{name} {step}: result (v1 source compiled as v2)");
    assert_eq!(a.fuel_used, c.fuel_used, "{name} {step}: fuel (v1 source compiled as v2)");
    assert_eq!(ha.storage, hb.storage, "{name} {step}: storage");
    assert_eq!(ha.storage, hc.storage, "{name} {step}: storage (v2)");
    assert_eq!(ha.events, hb.events, "{name} {step}: events");
    assert_eq!(ha.events, hc.events, "{name} {step}: events (v2)");
    assert_eq!((ha.balance, &ha.sent, ha.destroyed), (hb.balance, &hb.sent, hb.destroyed), "{name} {step}: money");
    assert_eq!((ha.balance, &ha.sent, ha.destroyed), (hc.balance, &hc.sent, hc.destroyed), "{name} {step}: money (v2)");
}

#[test]
fn values_and_types_decode_identically() {
    let mut rng = Rng(77);
    for _ in 0..5000 {
        let bytes: Vec<u8> = (0..rng.below(64)).map(|_| rng.next() as u8).collect();
        let a = borsh::from_slice::<tccl_v1::Value>(&bytes).ok().map(|v| borsh::to_vec(&v).unwrap());
        let b = borsh::from_slice::<tccl::Value>(&bytes).ok().map(|v| borsh::to_vec(&v).unwrap());
        assert_eq!(a, b);
    }
    for t in ["int", "list[bytes]", "map[address, list[int]]", "nothing", "list[list[text]]", "map[int]", "bad"] {
        assert_eq!(t.parse::<tccl_v1::Type>().map(|x| x.to_string()), t.parse::<tccl::Type>().map(|x| x.to_string()));
    }
}
