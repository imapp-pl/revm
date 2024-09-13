use clap::Parser;
use revm::{
    db::{BenchmarkDB, EthereumBenchmarkWiring},
    inspector_handle_register,
    inspectors::TracerEip3155,
    interpreter::{
        analysis::to_analysed, opcode::make_instruction_table, Contract, DummyHost, Interpreter,
        EMPTY_SHARED_MEMORY,
    },
    primitives::{
        address, Address, Bytecode, BytecodeDecodeError, Bytes, EthereumWiring, LatestSpec, TxKind,
    },
    Database, Evm,
};
use std::io::Error as IoError;
use std::path::PathBuf;
use std::{borrow::Cow, fs};

#[derive(Debug, thiserror::Error)]
pub enum Errors {
    #[error("The specified path does not exist")]
    PathNotExists,
    #[error("Invalid bytecode")]
    InvalidBytecode,
    #[error("Invalid input")]
    InvalidInput,
    #[error("EVM Error")]
    EVMError,
    #[error(transparent)]
    Io(#[from] IoError),
    #[error(transparent)]
    BytecodeDecodeError(#[from] BytecodeDecodeError),
}

/// Evm runner command allows running arbitrary evm bytecode.
/// Bytecode can be provided from cli or from file with --path option.
#[derive(Parser, Debug)]
pub struct Cmd {
    /// Hex-encoded EVM bytecode to be executed.
    #[arg(required_unless_present = "path")]
    bytecode: Option<String>,
    /// Path to a file containing the hex-encoded EVM bytecode to be executed.
    /// Overrides the positional `bytecode` argument.
    #[arg(long)]
    path: Option<PathBuf>,
    /// Run in benchmarking mode.
    #[arg(long)]
    bench: bool,
    /// Hex-encoded input/calldata bytes.
    #[arg(long, default_value = "")]
    input: String,
    /// Print the state.
    #[arg(long)]
    state: bool,
    /// Print the trace.
    #[arg(long)]
    trace: bool,
}

impl Cmd {
    /// Run evm runner command.
    pub fn run(&self) -> Result<(), Errors> {
        const CALLER: Address = address!("0000000000000000000000000000000000000001");

        let bytecode_str: Cow<'_, str> = if let Some(path) = &self.path {
            // check if path exists.
            if !path.exists() {
                return Err(Errors::PathNotExists);
            }
            fs::read_to_string(path)?.into()
        } else if let Some(bytecode) = &self.bytecode {
            bytecode.as_str().into()
        } else {
            unreachable!()
        };

        let bytecode = hex::decode(bytecode_str.trim()).map_err(|_| Errors::InvalidBytecode)?;
        let input = hex::decode(self.input.trim())
            .map_err(|_| Errors::InvalidInput)?
            .into();

        if self.bench {
            run_benchmark(bytecode_str);
            return Ok(());
        }

        let mut db = BenchmarkDB::new_bytecode(Bytecode::new_raw_checked(bytecode.into())?);

        let nonce = db.basic(CALLER).unwrap().map_or(0, |account| account.nonce);

        // BenchmarkDB is dummy state that implements Database trait.
        // the bytecode is deployed at zero address.
        let mut evm = Evm::<EthereumWiring<BenchmarkDB, TracerEip3155>>::builder()
            .with_db(db)
            .modify_tx_env(|tx| {
                // execution globals block hash/gas_limit/coinbase/timestamp..
                tx.caller = CALLER;
                tx.transact_to = TxKind::Call(Address::ZERO);
                tx.data = input;
                tx.nonce = nonce;
            })
            .with_external_context(TracerEip3155::new(Box::new(std::io::stdout())))
            .build();

        let out = if self.trace {
            let mut evm = evm
                .modify()
                .append_handler_register(inspector_handle_register)
                .build();

            evm.transact().map_err(|_| Errors::EVMError)?
        } else {
            let out = evm.transact().map_err(|_| Errors::EVMError)?;
            println!("Result: {:#?}", out.result);
            out
        };

        if self.state {
            println!("State: {:#?}", out.state);
        }

        Ok(())
    }
}

fn run_benchmark(bytecode_str: Cow<str>) {
    let evm = Evm::<EthereumBenchmarkWiring>::builder()
        .with_db(BenchmarkDB::new_bytecode(Bytecode::new()))
        .modify_tx_env(|tx| {
            tx.caller = "1000000000000000000000000000000000000000".parse().unwrap();
            tx.transact_to =
                TxKind::Call("0000000000000000000000000000000000000000".parse().unwrap());
        })
        .with_external_context(())
        .build();

    let host = DummyHost::new(*evm.context.evm.env.clone());
    let instruction_table =
        make_instruction_table::<DummyHost<EthereumWiring<BenchmarkDB, ()>>, LatestSpec>();
    let contract = Contract {
        input: Bytes::from(hex::decode("").unwrap()),
        bytecode: to_analysed(Bytecode::new_raw(
            hex::decode(bytecode_str.to_string()).unwrap().into(),
        )),
        ..Default::default()
    };
    let mut criterion = criterion::Criterion::default();

    let mut criterion_group = criterion.benchmark_group("revme");
    criterion_group.bench_function("bytecode", |b| {
        b.iter(|| {
            Interpreter::new(contract.clone(), u64::MAX, false).run(
                EMPTY_SHARED_MEMORY,
                &instruction_table,
                &mut host.clone(),
            );
        })
    });
    criterion_group.finish();
}
