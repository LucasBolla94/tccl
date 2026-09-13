//! Language version 2: permissions, typed records and transitions, calls between
//! contracts, re-entrancy, atomicity, limits, upgrades and the standard modules.

use tccl::program::Value;
use tccl::sim::{account, DeployOptions, Simulator};
use tccl::{compile, CompileOptions, VmError};

const TCN: u64 = 100_000_000;

fn src(name: &str) -> String {
    std::fs::read_to_string(format!("{}/../../examples/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn int(v: i128) -> Value {
    Value::Int(v)
}

fn addr(name: &str) -> Value {
    Value::Address(account(name))
}

fn contract(hex_addr: &str) -> Value {
    let mut a = [0u8; 20];
    a.copy_from_slice(&hex::decode(hex_addr).unwrap());
    Value::Address(a)
}

fn deploy(sim: &mut Simulator, source: &str, who: &str, args: Vec<Value>) -> String {
    let (c, r) = sim.deploy(source, account(who), args, 0).unwrap();
    r.result.unwrap_or_else(|e| panic!("deploy failed: {e}"));
    c
}

fn ok(sim: &mut Simulator, c: &str, who: &str, f: &str, args: Vec<Value>, value: u64) -> Value {
    sim.call(c, account(who), f, args, value).unwrap().result.unwrap_or_else(|e| panic!("{f} failed: {e}"))
}

fn fails(sim: &mut Simulator, c: &str, who: &str, f: &str, args: Vec<Value>, value: u64) -> VmError {
    match sim.call(c, account(who), f, args, value).unwrap().result {
        Ok(v) => panic!("{f} should fail, returned {v:?}"),
        Err(e) => e,
    }
}

fn view(sim: &mut Simulator, c: &str, f: &str, args: Vec<Value>) -> Value {
    sim.view(c, f, args).unwrap().result.unwrap_or_else(|e| panic!("view {f} failed: {e}"))
}

#[test]
fn roles_and_only() {
    let mut sim = Simulator::new();
    let c = deploy(&mut sim, &src("cloud_coin.tccl"), "issuer", vec![int(1_000)]);
    assert_eq!(view(&mut sim, &c, "balance_of", vec![addr("issuer")]), int(1_000));
    let e = fails(&mut sim, &c, "mallory", "mint", vec![addr("mallory"), int(5)], 0);
    assert_eq!(e, VmError::Require("only minter can call 'mint'".into()));
    let r = sim.call(&c, account("issuer"), "add_minter", vec![addr("bob")], 0).unwrap();
    r.result.unwrap();
    assert_eq!(r.events[0].name, "RoleGranted");
    ok(&mut sim, &c, "bob", "mint", vec![addr("carol"), int(7)], 0);
    assert_eq!(view(&mut sim, &c, "total_supply_of", vec![]), int(1_007));
    // The standard token actions.
    ok(&mut sim, &c, "carol", "transfer", vec![addr("dave"), int(3)], 0);
    assert!(matches!(fails(&mut sim, &c, "carol", "transfer", vec![addr("dave"), int(5)], 0), VmError::Require(m) if m.contains("insufficient")));
    ok(&mut sim, &c, "dave", "approve", vec![addr("erin"), int(2)], 0);
    ok(&mut sim, &c, "erin", "transfer_from", vec![addr("dave"), addr("erin"), int(2)], 0);
    assert!(fails(&mut sim, &c, "erin", "transfer_from", vec![addr("dave"), addr("erin"), int(1)], 0).to_string().contains("allowance"));
}

#[test]
fn records_and_transitions() {
    let mut sim = Simulator::new();
    let c = deploy(&mut sim, &src("orders.tccl"), "boss", vec![]);
    ok(&mut sim, &c, "boss", "set_price", vec![Value::Text("lamp".into()), int(5 * TCN as i128)], 0);
    ok(&mut sim, &c, "boss", "hire_shipper", vec![addr("sam")], 0);
    assert_eq!(ok(&mut sim, &c, "ann", "place", vec![Value::Text("lamp".into())], 0), int(1));
    // Placed -> Delivered is not an allowed transition.
    let e = fails(&mut sim, &c, "ann", "confirm", vec![int(1)], 0);
    assert_eq!(e, VmError::TransitionNotAllowed { enum_name: "Status".into(), from: "Placed".into(), to: "Delivered".into() });
    assert!(fails(&mut sim, &c, "ann", "pay", vec![int(1)], TCN).to_string().contains("wrong amount"));
    ok(&mut sim, &c, "ann", "pay", vec![int(1)], 5 * TCN);
    assert!(fails(&mut sim, &c, "ann", "ship", vec![int(1)], 0).to_string().contains("only shipper"));
    ok(&mut sim, &c, "sam", "ship", vec![int(1)], 0);
    assert!(fails(&mut sim, &c, "ann", "cancel", vec![int(1)], 0).to_string().contains("Shipped to Cancelled"));
    ok(&mut sim, &c, "ann", "confirm", vec![int(1)], 0);
    let p = sim.program(&c).unwrap().clone();
    let o = view(&mut sim, &c, "order", vec![int(1)]);
    let (_, f) = p.find("order").unwrap();
    assert_eq!(tccl::abi::display_typed(&p, &o, &f.ret, "tcr"), format!("Order(buyer: {}, item: \"lamp\", price: 500000000, status: Delivered)", tccl::abi::format_address(&account("ann"), "tcr")));
    ok(&mut sim, &c, "boss", "fire_shipper", vec![addr("sam")], 0);
    let state = sim.describe_state(&c, "tcr");
    let shipper = state.iter().find(|s| s.name == "shipper").unwrap();
    assert_eq!((shipper.ty.as_str(), shipper.value.as_str()), ("role", ""));
}

const CALLER_ID: &str = r#"
contract Echo
event Seen(caller: address, origin: address)
action who() -> address:
    emit Seen(caller, origin)
    return caller
view double(x: int) -> int:
    return x * 2
action fail():
    require false, "callee failed"
action take() payable:
    pass
"#;

const PROXY: &str = r#"
contract Proxy
interface Echo:
    action who() -> address
    view double(x: int) -> int
    action fail()
    action take() payable
state calls: int
action ask(target: address) -> address:
    calls += 1
    return Echo(target).who()
action ask_fail(target: address):
    calls += 1
    Echo(target).fail()
action pay(target: address, amount: int) payable:
    Echo(target).take() with value amount
view twice(target: address, x: int) -> int:
    return Echo(target).double(x)
"#;

#[test]
fn caller_identity_atomicity_and_values() {
    let mut sim = Simulator::new();
    let echo = deploy(&mut sim, CALLER_ID, "alice", vec![]);
    let proxy = deploy(&mut sim, PROXY, "alice", vec![]);
    let r = sim.call(&proxy, account("bob"), "ask", vec![contract(&echo)], 0).unwrap();
    assert_eq!(r.result.unwrap(), contract(&proxy), "the callee sees the calling contract as caller");
    assert_eq!(r.events[0].fields, vec![("caller".into(), contract(&proxy)), ("origin".into(), addr("bob"))]);
    assert_eq!(r.events[0].contract, echo, "events are attributed to the contract that emits them");
    assert_eq!(view(&mut sim, &proxy, "twice", vec![contract(&echo), int(21)]), int(42));

    // A failure in the callee reverts the caller's changes too.
    let before = sim.describe_state(&proxy, "tcr");
    let e = fails(&mut sim, &proxy, "bob", "ask_fail", vec![contract(&echo)], 0);
    assert_eq!(e, VmError::Require("callee failed".into()));
    assert_eq!(sim.describe_state(&proxy, "tcr"), before);

    // Value forwarded with `with value` moves from the proxy balance to the callee.
    let bob = sim.balance_of(&account("bob"));
    ok(&mut sim, &proxy, "bob", "pay", vec![contract(&echo), int(3 * TCN as i128)], 5 * TCN);
    assert_eq!(sim.balance_of(&account("bob")), bob - 5 * TCN);
    assert_eq!(sim.balances[&proxy], 2 * TCN);
    assert_eq!(sim.balances[&echo], 3 * TCN);
    // Paying more than the proxy holds fails and reverts the whole call, including bob's payment.
    let bob = sim.balance_of(&account("bob"));
    assert_eq!(fails(&mut sim, &proxy, "bob", "pay", vec![contract(&echo), int(10 * TCN as i128)], TCN), VmError::InsufficientBalance);
    assert_eq!(sim.balance_of(&account("bob")), bob);

    // Calling an address without a contract.
    assert!(matches!(fails(&mut sim, &proxy, "bob", "ask", vec![addr("nobody")], 0), VmError::NoContract(_)));
}

#[test]
fn reentrancy_is_always_rejected() {
    let a = r#"
contract A
interface B:
    action ping(back: address)
state hits: int
action start(b: address):
    hits += 1
    B(b).ping(self)
action pong():
    hits += 100
"#;
    let b = r#"
contract B
interface A:
    action pong()
action ping(back: address):
    A(back).pong()
"#;
    let mut sim = Simulator::new();
    let ca = deploy(&mut sim, a, "alice", vec![]);
    let cb = deploy(&mut sim, b, "alice", vec![]);
    let e = fails(&mut sim, &ca, "alice", "start", vec![contract(&cb)], 0);
    assert_eq!(e, VmError::Reentrancy(ca.clone()));
    assert_eq!(sim.describe_state(&ca, "tcr")[0].value, "0", "hits reverted");
    // A contract calling itself through an interface is also re-entrancy.
    let selfcall = "contract S\ninterface S2:\n    action f()\naction f():\n    S2(self).f()\n";
    let cs = deploy(&mut sim, selfcall, "alice", vec![]);
    assert!(matches!(fails(&mut sim, &cs, "alice", "f", vec![], 0), VmError::Reentrancy(_)));
}

#[test]
fn interface_mismatch_depth_and_destroy() {
    let callee = "contract C\naction f(x: text) -> int:\n    return 1\naction bye(to: address):\n    destroy(to)\n";
    let caller = r#"
contract D
interface C:
    action f(x: int) -> int
    action bye(to: address)
action go(c: address) -> int:
    return C(c).f(1)
action kill(c: address):
    C(c).bye(self)
"#;
    let mut sim = Simulator::new();
    let c = deploy(&mut sim, callee, "alice", vec![]);
    let d = deploy(&mut sim, caller, "alice", vec![]);
    assert!(matches!(fails(&mut sim, &d, "alice", "go", vec![contract(&c)], 0), VmError::InterfaceMismatch(m) if m.contains("(text) -> int")));
    assert_eq!(fails(&mut sim, &d, "alice", "kill", vec![contract(&c)], 0), VmError::DestroyInNestedCall);

    // A chain of contracts each calling the next one.
    let link = "contract L\ninterface L2:\n    action hop(rest: list[address]) -> int\naction hop(rest: list[address]) -> int:\n    if len(rest) == 0:\n        return 1\n    let next: list[address] = []\n    for i in range(1, len(rest)):\n        next.push(rest[i])\n    return 1 + L2(rest[0]).hop(next)\n";
    let links: Vec<String> = (0..10).map(|_| deploy(&mut sim, link, "alice", vec![])).collect();
    let chain = |n: usize| Value::List(links[1..n].iter().map(|l| contract(l)).collect());
    assert_eq!(ok(&mut sim, &links[0], "alice", "hop", vec![chain(7)], 0), int(7));
    assert_eq!(fails(&mut sim, &links[0], "alice", "hop", vec![chain(10)], 0), VmError::ContractDepth);
}

#[test]
fn dex_pool_swaps_through_token_contracts() {
    let mut sim = Simulator::new();
    let coin = src("cloud_coin.tccl");
    let a = deploy(&mut sim, &coin, "lp", vec![int(1_000_000)]);
    let b = deploy(&mut sim, &coin, "lp", vec![int(1_000_000)]);
    let pool = deploy(&mut sim, &src("pool.tccl"), "lp", vec![contract(&a), contract(&b)]);
    ok(&mut sim, &a, "lp", "approve", vec![contract(&pool), int(100_000)], 0);
    ok(&mut sim, &b, "lp", "approve", vec![contract(&pool), int(400_000)], 0);
    assert_eq!(ok(&mut sim, &pool, "lp", "add_liquidity", vec![int(100_000), int(400_000)], 0), int(316 * 632));
    // A trader buys B with A.
    ok(&mut sim, &a, "lp", "transfer", vec![addr("trader"), int(10_000)], 0);
    ok(&mut sim, &a, "trader", "approve", vec![contract(&pool), int(10_000)], 0);
    let quote = view(&mut sim, &pool, "quote", vec![Value::Bool(true), int(10_000)]);
    // 10 000 × 0.997 × 400 000 ÷ (100 000 + 9 970) = 36 264
    assert_eq!(quote, int(36_264));
    assert!(fails(&mut sim, &pool, "trader", "swap", vec![Value::Bool(true), int(10_000), int(40_000)], 0).to_string().contains("min_out"));
    assert_eq!(ok(&mut sim, &pool, "trader", "swap", vec![Value::Bool(true), int(10_000), int(36_000)], 0), int(36_264));
    assert_eq!(view(&mut sim, &b, "balance_of", vec![addr("trader")]), int(36_264));
    assert_eq!(view(&mut sim, &a, "balance_of", vec![contract(&pool)]), int(110_000));
    // Without an allowance the token call fails and nothing changes in the pool.
    let reserves = view(&mut sim, &pool, "reserves", vec![]);
    assert!(fails(&mut sim, &pool, "trader", "swap", vec![Value::Bool(true), int(1_000), int(1)], 0).to_string().contains("allowance"));
    assert_eq!(view(&mut sim, &pool, "reserves", vec![]), reserves);
    ok(&mut sim, &pool, "lp", "remove_liquidity", vec![int(1_000)], 0);
}

#[test]
fn coin_flip_commit_reveal() {
    let mut sim = Simulator::new();
    let c = deploy(&mut sim, &src("coin_flip.tccl"), "house", vec![]);
    let secret = vec![7u8; 32];
    let mut preimage = secret.clone();
    preimage.push(1); // choice = true
    let commitment = <sha2::Sha256 as sha2::Digest>::digest(&preimage).to_vec();
    ok(&mut sim, &c, "host", "create", vec![Value::Bytes(commitment)], 2 * TCN);
    assert!(fails(&mut sim, &c, "guest", "join", vec![int(1), Value::Bool(true)], TCN).to_string().contains("stake must match"));
    ok(&mut sim, &c, "guest", "join", vec![int(1), Value::Bool(true)], 2 * TCN);
    assert!(fails(&mut sim, &c, "host", "reveal", vec![int(1), Value::Bytes(secret.clone()), Value::Bool(false)], 0).to_string().contains("does not match"));
    let guest = sim.balance_of(&account("guest"));
    ok(&mut sim, &c, "host", "reveal", vec![int(1), Value::Bytes(secret), Value::Bool(true)], 0);
    assert_eq!(sim.balance_of(&account("guest")), guest + 4 * TCN);
    assert_eq!(view(&mut sim, &c, "stage_of", vec![int(1)]), Value::Text("Settled".into()));
}

#[test]
fn tickets_and_conditional_payments() {
    let mut sim = Simulator::new();
    let t = deploy(&mut sim, &src("tickets.tccl"), "org", vec![int(2)]);
    ok(&mut sim, &t, "org", "issue", vec![addr("ann"), Value::Text("A1".into())], 0);
    ok(&mut sim, &t, "org", "issue", vec![addr("ben"), Value::Text("A2".into())], 0);
    assert!(fails(&mut sim, &t, "org", "issue", vec![addr("cat"), Value::Text("A3".into())], 0).to_string().contains("sold out"));
    assert!(fails(&mut sim, &t, "ann", "transfer_item", vec![int(2), addr("ann")], 0).to_string().contains("only the owner"));
    ok(&mut sim, &t, "ann", "transfer_item", vec![int(1), addr("cat")], 0);
    assert_eq!(view(&mut sim, &t, "owner_of", vec![int(1)]), addr("cat"));

    let d = deploy(&mut sim, &src("deals.tccl"), "anyone", vec![]);
    let secret = b"open sesame".to_vec();
    let lock = <sha2::Sha256 as sha2::Digest>::digest(&secret).to_vec();
    let args = |lock: Vec<u8>, release: i128, refund: i128| vec![addr("seller"), addr("judge"), int(release), int(refund), Value::Bytes(lock)];
    ok(&mut sim, &d, "buyer", "create_payment", args(lock, 0, 0), 5 * TCN);
    let seller = sim.balance_of(&account("seller"));
    assert!(fails(&mut sim, &d, "thief", "reveal_payment", vec![int(1), Value::Bytes(b"guess".to_vec())], 0).to_string().contains("wrong secret"));
    ok(&mut sim, &d, "thief", "reveal_payment", vec![int(1), Value::Bytes(secret)], 0);
    assert_eq!(sim.balance_of(&account("seller")), seller + 5 * TCN, "the secret pays the payee, whoever reveals it");
    assert!(fails(&mut sim, &d, "buyer", "refund_payment", vec![int(1)], 0).to_string().contains("no longer pending"));

    ok(&mut sim, &d, "buyer", "create_payment", args(vec![], 0, 50), 3 * TCN);
    assert!(fails(&mut sim, &d, "buyer", "refund_payment", vec![int(2)], 0).to_string().contains("not allowed yet"));
    sim.height = 60;
    let buyer = sim.balance_of(&account("buyer"));
    ok(&mut sim, &d, "buyer", "refund_payment", vec![int(2)], 0);
    assert_eq!(sim.balance_of(&account("buyer")), buyer + 3 * TCN);
    assert_eq!(view(&mut sim, &d, "payment_status", vec![int(2)]), Value::Text("Refunded".into()));
    assert_eq!(view(&mut sim, &d, "locked_total", vec![]), int(0));
}

#[test]
fn upgrades_follow_the_authority() {
    let mut sim = Simulator::new();
    let (c, r) = sim.deploy(&src("counter.tccl"), account("dev"), vec![], 0).unwrap();
    r.result.unwrap();
    ok(&mut sim, &c, "bob", "increment", vec![int(5)], 0);
    let v2 = src("counter_v2.tccl");
    assert!(sim.upgrade(&c, account("bob"), &v2, vec![]).unwrap_err().contains("only the upgrade authority"));
    let bad = "contract Counter\nstate count: text\nstate last_caller: address\naction f():\n    pass\n";
    assert!(sim.upgrade(&c, account("dev"), bad, vec![]).unwrap_err().contains("incompatible"));
    let (report, r) = sim.upgrade(&c, account("dev"), &v2, vec![]).unwrap();
    r.result.unwrap();
    assert_eq!(report.added_state, ["step"]);
    assert!(report.runs_upgrade_hook);
    ok(&mut sim, &c, "bob", "increment", vec![int(2)], 0);
    assert_eq!(view(&mut sim, &c, "get", vec![]), int(25), "old state kept, new code active");

    // A failing upgrade() keeps the old code.
    let failing = "contract Counter\nstate count: int\nstate last_caller: address\nstate step: int\nupgrade():\n    require false, \"not today\"\nview get() -> int:\n    return 0\n";
    let (_, r) = sim.upgrade(&c, account("dev"), failing, vec![]).unwrap();
    assert!(r.result.is_err());
    assert_eq!(view(&mut sim, &c, "get", vec![]), int(25));

    // Other contracts can check whether a dependency can still change.
    let checker = "contract Check\naction final_code(c: address) -> bool:\n    return is_final(c)\naction hash(c: address) -> bytes:\n    return code_hash(c)\n";
    let k = deploy(&mut sim, checker, "dev", vec![]);
    assert_eq!(ok(&mut sim, &k, "dev", "final_code", vec![contract(&c)], 0), Value::Bool(false));
    assert!(sim.set_authority(&c, account("bob"), None).is_err());
    sim.set_authority(&c, account("dev"), None).unwrap();
    assert_eq!(ok(&mut sim, &k, "dev", "final_code", vec![contract(&c)], 0), Value::Bool(true));
    assert!(sim.upgrade(&c, account("dev"), &v2, vec![]).unwrap_err().contains("final"));
    assert_eq!(ok(&mut sim, &k, "dev", "hash", vec![contract(&c)], 0), Value::Bytes(sim.program(&c).unwrap().code_hash().to_vec()));
    assert_eq!(ok(&mut sim, &k, "dev", "hash", vec![addr("nobody")], 0), Value::Bytes(vec![]));

    let (f, _) = sim.deploy_with(&src("counter.tccl"), account("dev"), vec![], 0, &DeployOptions { final_code: true, ..Default::default() }).unwrap();
    assert!(sim.upgrade(&f, account("dev"), &v2, vec![]).is_err());
}

#[test]
fn memory_limit_applies_to_version_two_only() {
    let hog = "contract Hog\naction eat() -> int:\n    let big: text = \"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\"\n    for i in range(0, 9):\n        big += big\n    let xs: list[text] = []\n    for i in range(0, 1000):\n        xs.push(big)\n    return len(xs)\n";
    let mut sim = Simulator::new();
    let v2 = deploy(&mut sim, hog, "alice", vec![]);
    let r = sim.call(&v2, account("alice"), "eat", vec![], 0).unwrap();
    assert!(matches!(r.result, Err(VmError::MemoryLimit(_))), "{:?}", r.result);
    let (v1, d) = sim.deploy_with(hog, account("alice"), vec![], 0, &DeployOptions { language: 1, ..Default::default() }).unwrap();
    d.result.unwrap();
    let r = sim.call(&v1, account("alice"), "eat", vec![], 0).unwrap();
    assert_eq!(r.result, Ok(int(1000)), "version 1 behaviour is unchanged");
}

#[test]
fn out_of_fuel_in_a_nested_call_reverts_everything() {
    let spender = "contract Burner\naction burn():\n    let i: int = 0\n    while true:\n        i += 1\n";
    let caller = "contract Outer\ninterface Burner:\n    action burn()\nstate n: int\naction go(b: address):\n    n = 7\n    Burner(b).burn()\n";
    let mut sim = Simulator::new();
    let b = deploy(&mut sim, spender, "alice", vec![]);
    let o = deploy(&mut sim, caller, "alice", vec![]);
    let r = sim.call(&o, account("alice"), "go", vec![contract(&b)], 0).unwrap();
    assert_eq!(r.result, Err(VmError::OutOfFuel));
    assert_eq!(r.fuel_used, tccl::sim::DEFAULT_FUEL);
    assert_eq!(sim.describe_state(&o, "tcr")[0].value, "0");
}

#[test]
fn hostile_input_never_panics() {
    let mut seed = 0x9e3779b97f4a7c15u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let alphabet: Vec<char> = "contract module use record enum interface role only grant revoke to from with value upgrade action view fn init state event let if else while for in return require send emit ( ) [ ] { } : , . -> = += == != < > + - * / % \n    \"x\" 0x12 123 a b c Order Phase.Open token.mint".split(' ').flat_map(|w| format!("{w} ").chars().collect::<Vec<_>>()).collect();
    let words: Vec<&str> = "contract module use record enum interface role only grant revoke to from with value upgrade action view fn init state event let if else while for in return require send emit ( ) [ ] : , . -> = += == != < > + - * / % \n \n    \"x\" 0x12 123 a b c Order Phase.Open token.mint std.token int text address list map".split(' ').collect();
    let _ = alphabet;
    let base = std::fs::read_to_string(format!("{}/../../examples/orders.tccl", env!("CARGO_MANIFEST_DIR"))).unwrap();
    for round in 0..3000 {
        let src = if round % 2 == 0 {
            (0..(next() % 80)).map(|_| words[(next() % words.len() as u64) as usize]).collect::<Vec<_>>().join(" ")
        } else {
            // Mutate a real contract: delete, duplicate or replace a random slice.
            let mut s: Vec<char> = base.chars().collect();
            for _ in 0..(1 + next() % 4) {
                let i = (next() % s.len() as u64) as usize;
                let j = (i + (next() % 20) as usize).min(s.len());
                match next() % 3 {
                    0 => {
                        s.drain(i..j);
                    }
                    1 => {
                        let piece: Vec<char> = s[i..j].to_vec();
                        s.splice(i..i, piece);
                    }
                    _ => {
                        let w = words[(next() % words.len() as u64) as usize];
                        s.splice(i..j, w.chars());
                    }
                }
                if s.is_empty() {
                    break;
                }
            }
            s.into_iter().collect()
        };
        for version in [1, 2] {
            let _ = compile(&src, &CompileOptions { version, ..Default::default() });
        }
    }
    // Random bytes as a program never panic when decoded.
    for _ in 0..2000 {
        let bytes: Vec<u8> = (0..(next() % 300)).map(|_| next() as u8).collect();
        let _ = tccl::Program::from_bytes(&bytes);
        let mut with_version = vec![2u8, 0];
        with_version.extend(&bytes);
        let _ = tccl::Program::from_bytes(&with_version);
    }
}

#[test]
fn programs_round_trip_and_version_one_bytes_are_stable() {
    for f in std::fs::read_dir(format!("{}/../../examples", env!("CARGO_MANIFEST_DIR"))).unwrap() {
        let path = f.unwrap().path();
        if path.extension().is_none_or(|x| x != "tccl") {
            continue;
        }
        let s = std::fs::read_to_string(&path).unwrap();
        let p = compile(&s, &CompileOptions::default()).unwrap();
        assert_eq!(tccl::Program::from_bytes(&p.to_bytes()).unwrap(), p, "{}", path.display());
        if let Ok(p1) = compile(&s, &CompileOptions { version: 1, ..Default::default() }) {
            let bytes = p1.to_bytes();
            assert_eq!(&bytes[..2], &[1, 0]);
            assert_eq!(tccl::Program::from_bytes(&bytes).unwrap(), p1);
        }
    }
}

#[test]
fn language_reference_claims() {
    let src = r#"
contract Claims
enum Color: Red, Green, Blue
record Point:
    x: int
    tag: text
state by_color: map[Color, int]
state origin_point: Point
state fav: Color = Color.Green
event Moved(p: Point, c: Color)
interface Other:
    view ping() -> int

action paint(c: Color, n: int):
    by_color[c] += n

action move(p: Point) payable -> Point:
    origin_point = p
    emit Moved(p, fav)
    return origin_point

action reset():
    origin_point = Point(x: 0, tag: "")

view count(c: Color) -> int:
    return by_color[c]

view same(a: address) -> bool:
    return Other(a) == a

view name(c: Color) -> text:
    return to_text(c)
"#;
    let mut sim = Simulator::new();
    let c = deploy(&mut sim, src, "alice", vec![]);
    ok(&mut sim, &c, "alice", "paint", vec![int(2), int(5)], 0);
    assert_eq!(view(&mut sim, &c, "count", vec![int(2)]), int(5));
    let p = Value::List(vec![int(3), Value::Text("a".into())]);
    let r = sim.call(&c, account("alice"), "move", vec![p.clone()], 1).unwrap();
    assert_eq!(r.result.unwrap(), p);
    assert_eq!(r.events[0].fields[1].1, int(1));
    let before = sim.contracts[&c].storage.len();
    ok(&mut sim, &c, "alice", "reset", vec![], 0);
    assert_eq!(sim.contracts[&c].storage.len(), before - 1, "storing the default record deletes the entry");
    assert_eq!(view(&mut sim, &c, "same", vec![addr("x")]), Value::Bool(true));
    assert_eq!(view(&mut sim, &c, "name", vec![int(0)]), Value::Text("Red".into()));
    // Version 1 names that are contextual words in version 2 still compile.
    let legacy = "contract L\nstate record: int\naction upgrade_it(to: address, from: address, value2: int):\n    record += value2\nfn only() -> int:\n    return 1\n";
    compile(legacy, &CompileOptions { version: 1, ..Default::default() }).unwrap();
    compile(legacy, &CompileOptions::default()).unwrap();
}

#[test]
fn version_one_contracts_cannot_be_called_and_hardening_is_opt_in() {
    let mut sim = Simulator::new();
    let (old, d) = sim.deploy_with(&src("counter.tccl"), account("dev"), vec![], 0, &DeployOptions { language: 1, ..Default::default() }).unwrap();
    d.result.unwrap();
    let caller = "contract C\ninterface Counter:\n    action increment(amount: int)\naction poke(c: address):\n    Counter(c).increment(1)\n";
    let c = deploy(&mut sim, caller, "dev", vec![]);
    assert_eq!(fails(&mut sim, &c, "dev", "poke", vec![contract(&old)], 0), VmError::Unsupported("calls into language version 1 contracts".into()));

    // The same version 1 program with and without the network's hardening switch.
    let src = "contract B\naction run(n: int) -> int:\n    let xs: list[text] = []\n    for i in range(0, 4000):\n        xs.push(\"a\")\n    let c: int = 0\n    for i in range(0, n):\n        c += len(xs)\n    return c\n";
    let p = compile(src, &CompileOptions { version: 1, ..Default::default() }).unwrap();
    #[derive(Default)]
    struct Mem(std::collections::BTreeMap<Vec<u8>, Vec<u8>>);
    impl tccl::vm::Host for Mem {
        fn storage_read(&mut self, k: &[u8]) -> Result<Option<Vec<u8>>, VmError> {
            Ok(self.0.get(k).cloned())
        }
        fn storage_write(&mut self, k: &[u8], v: Option<Vec<u8>>) -> Result<(), VmError> {
            match v {
                Some(v) => self.0.insert(k.to_vec(), v),
                None => self.0.remove(k),
            };
            Ok(())
        }
        fn balance(&mut self) -> Result<u64, VmError> {
            Ok(0)
        }
        fn send(&mut self, _: &[u8; 20], _: u64) -> Result<(), VmError> {
            Ok(())
        }
        fn emit(&mut self, _: &str, _: Vec<(String, Value)>) -> Result<(), VmError> {
            Ok(())
        }
        fn storage_items(&mut self) -> Result<u64, VmError> {
            Ok(0)
        }
        fn destroy(&mut self, _: &[u8; 20]) -> Result<(), VmError> {
            Ok(())
        }
    }
    let ctx = tccl::vm::CallContext { caller: [1; 20], value: 0, height: 1, self_address: [2; 20] };
    let plain = tccl::vm::execute(&p, tccl::vm::Mode::Action, "run", vec![int(2000)], &ctx, &mut Mem::default(), 10_000_000);
    let hard = tccl::vm::execute_with(&p, tccl::vm::Mode::Action, "run", vec![int(2000)], &ctx, &mut Mem::default(), 10_000_000, tccl::vm::ExecOptions { harden_v1: true });
    assert_eq!(plain.result, hard.result);
    assert!(plain.fuel_used > hard.fuel_used, "len(xs) no longer copies the list: {} vs {}", plain.fuel_used, hard.fuel_used);
}
