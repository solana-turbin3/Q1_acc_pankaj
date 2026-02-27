# Accel Pinocchio Escrow

A high-performance Escrow program built on Solana using the **[Pinocchio](https://github.com/anza-xyz/pinocchio)** framework. This repository demonstrates two implementations of the same escrow logic:
1. **V1 (Raw Pointers)**: Direct memory manipulation via raw pointers.
2. **V2 (Wincode)**: Zero-copy deserialization using the `wincode` crate.

This project was built to explore maximum performance (Compute Unit optimization) and safe zero-copy deserialization techniques on Solana.

---

##  Architecture

The program supports the standard Escrow workflow:

1. **Make**: Maker deposits `mint_a` tokens into a PDA vault and specifies the amount of `mint_b` tokens they want in return.
2. **Take**: Taker sends `mint_b` tokens directly to the maker, and receives the `mint_a` tokens from the PDA vault. The escrow and vault are then closed.
3. **Refund**: Maker cancels the escrow. The tokens in the vault are returned to the maker, and the escrow account is closed.

Both V1 and V2 support these three instructions.

---

##  V1 vs V2: The Quest for Zero-Copy

### V1: The Raw Approach

V1 relies on directly casting the account bytes to a struct reference. 
```rust
Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
```
While extremely efficient, this requires manual safety guarantees. Specifically, the data length and memory alignment must be perfectly calculated.

### V2: The Wincode Approach

V2 uses the `wincode` crate alongside `#[derive(SchemaWrite, SchemaRead)]` to achieve zero-copy deserialization safely. Wincode maps the raw bytes to the structured data seamlessly without manual pointer arithmetic.

* For on-chain state (`EscrowV2`), we still rely on strict memory layout via `#[repr(C)]`.
* For instruction parsing (`MakeParams`), Wincode handles typical byte parsing without alignment concerns, allowing us to use standard types like `u64`.

---

##  Memory Alignment & Padding Insights

One of the key engineering decisions in this repository was defining the Escrow state using `[u8; 8]` instead of `u64`. 

```rust
#[repr(C)]
pub struct Escrow {
    maker: [u8; 32],
    mint_a: [u8; 32],
    mint_b: [u8; 32],
    amount_to_receive: [u8; 8], // Why not u64?
    amount_to_give: [u8; 8],
    pub bump: u8,
}
```

### Why we didn't use `u64`
If we used `u64`, the `Escrow` struct's alignment requirement would immediately jump to **8 bytes**. 
Because a Solana account expects exactly 113 bytes of payload, the Rust compiler (due to `#[repr(C)]`) would add exactly **7 extra bytes of padding** at the end of the struct to make the total size divisible by 8 (120 bytes total). 

**The fix**: By using `[u8; 8]`, the alignment requirement drops back to **1 byte**, meaning zero padding is added, and the struct maps perfectly to the **113 byte** account logic, avoiding `InvalidAccountData` errors on-chain.

---

##  Benchmarks

A core part of this project was measuring the Compute Unit (CU) overhead of `wincode` vs Raw pointers.

Both methods compile down to extremely similar CPU instructions as they both leverage zero-copy concepts. The only minor deviation was on `Make` due to instruction data interpretation via `wincode::deserialize`.

```text
╔══════════════════════════════════════════════════════════╗
║        BENCHMARK: V1 (Raw Pointer) vs V2 (Wincode)       ║
║                 (5 iterations each)                      ║
╠══════════════════════════════════════════════════════════╣
║ Instruction │  V1 (CUs)  │  V2 (CUs)  │   Delta  │ Diff% ║
╠═════════════╪════════════╪════════════╪══════════╪═══════╣
║ Make        │      31952 │      32245 │     +293 │+0.92% ║
║ Take        │      16463 │      16463 │       +0 │+0.00% ║
║ Refund      │      10491 │      10492 │       +1 │+0.01% ║
╠═════════════╪════════════╪════════════╪══════════╪═══════╣
║ TOTAL       │      58906 │      59200 │     +294 │+0.50% ║
╚══════════════════════════════════════════════════════════╝
```
*\*The benchmark was run using LiteSVM. Run `cargo test benchmark_v1_vs_v2 -- --nocapture` to generate.*

---

##  Build & Test Instructions

### Prerequisites
- Install [Rust](https://rustup.rs/)
- Install the [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) (`cargo build-sbf` required)

### Build the Program

Because `pinocchio` optimizes for SBF, you must compile the program using `cargo-build-sbf`:

```bash
cargo-build-sbf
```

### Run Tests

The repository includes heavy end-to-end integration tests using `litesvm`. Tests are separated strictly from the main logic for clean abstractions.

```bash
cargo test -- --nocapture
```

To run exclusively the `wincode` vs raw pointer benchmark:

```bash
cargo test benchmark_v1_vs_v2 -- --nocapture
```
