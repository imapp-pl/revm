use bytes::Bytes;
use criterion::{criterion_group, criterion_main, Criterion};
use revm::{
    db::BenchmarkDB,
    interpreter::{analysis::to_analysed, BytecodeLocked, Contract, DummyHost, Interpreter},
    primitives::{LatestSpec, Bytecode, TransactTo}
};
use std::io;
use std::time::{Duration, Instant};

extern crate alloc;

pub fn criterion_benchmark(_c: &mut Criterion) {
    let mut code_input = String::new();
    let stdin = io::stdin();
    match stdin.read_line(&mut code_input){
        Ok(_n) => {}
        Err(_error) => {}
    }
    if code_input.trim().len() == 0 {
        println!("Empty bytecode on stdin.");
        return
    }
    let code_input = code_input.trim();
    let bytecode = to_analysed(Bytecode::new_raw(hex::decode(code_input).unwrap().into()));
    let stop_bytecode = to_analysed(Bytecode::new_raw(hex::decode("00".to_string() + code_input).unwrap().into()));

    // EVM preparation
    let mut evm = revm::new();
    evm.env.tx.caller = "0x1000000000000000000000000000000000000000"
        .parse()
        .unwrap();
    evm.env.tx.transact_to = TransactTo::Call(
        "0x0000000000000000000000000000000000000000"
            .parse()
            .unwrap(),
    );
    evm.env.tx.data = Bytes::from(hex::decode("").unwrap());
    evm.database(BenchmarkDB::new_bytecode(Bytecode::new()));

    let contract = Contract {
        input: Bytes::from(hex::decode("").unwrap()),
        bytecode: BytecodeLocked::try_from(bytecode).unwrap(),
        ..Default::default()
    };
    let stop_contract = Contract {
        input: Bytes::from(hex::decode("").unwrap()),
        bytecode: BytecodeLocked::try_from(stop_bytecode).unwrap(),
        ..Default::default()
    };

    Criterion::default()
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(200))
        .bench_function("bytecode-benchmark", |b| {
            b.iter_custom(|iters| {
                let mut dur = Duration::from_nanos(0);
                for _i in 0..iters {
                    let mut interpreter = Interpreter::new(Box::new(contract.clone()), u64::MAX, false);
                    let mut host = DummyHost::new(evm.env.clone());
                    let timer = Instant::now();
                    interpreter.run::<_, LatestSpec>(&mut host);
                    dur += timer.elapsed();
                }
                dur
            })
        });

    Criterion::default()
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_millis(200))
        .bench_function("bytecode-benchmark-stop", |b| {
            b.iter_custom(|iters| {
                let mut dur = Duration::from_nanos(0);
                for _i in 0..iters {
                    let mut interpreter = Interpreter::new(Box::new(stop_contract.clone()), u64::MAX, false);
                    let mut host = DummyHost::new(evm.env.clone());
                    let timer = Instant::now();
                    interpreter.run::<_, LatestSpec>(&mut host);
                    dur += timer.elapsed();
                }
                dur
            })
        });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
