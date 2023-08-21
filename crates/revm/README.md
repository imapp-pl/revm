# REVM benchmark

Benchmark goes in two runs. 
One is for passed bytecode. 
The second one is for bytecode prefixed with 00 code (STOP) - this is to determine the overhead from revm setup only.  

## How to run

Install Rust and Cargo:

`curl https://sh.rustup.rs -sSf | sh`

Pass bytecode string as stdin to benchmark:

`echo "6062" | cargo bench --bench criterion_bytecode`

For more verbose output:

`echo "6062" | cargo bench --bench criterion_bytecode -- --verbose`

## Debug
For debug information set environmental variable CRITERION_DEBUG=1.
