#  Pinocchio Fundraiser (Highly Optimized)

A hyper-optimized, low-CU Solana fundraiser program built using [Pinocchio](https://github.com/febo/pinocchio) and the `p-token` standard. This program demonstrates extreme Compute Unit (CU) minimization techniques and robust security practices for a production-ready Solana program.

##  Benchmark Results

By stripping away the heavy abstractions of Anchor and standard Solana SDKs, this program achieves incredibly low CU usage:

| Instruction | Compute Units |
| :--- | :--- |
| `Initialize` | **1,574 CU** |
| `CreateContributor` | **1,501 CU** |
| `Contribute` | **1,753 CU** |
| `CheckContributions` | **1,847 CU** |
| `Refund` | **2,132 CU** |

##  Optimizations & Architecture

This codebase implements several advanced optimization strategies to shatter the CU floor:

### 1. Lazy Entrypoint & Zero-Copy Parsing
Instead of parsing all accounts and instruction data upfront (which Anchor and standard SDKs do), we use Pinocchio's `lazy_program_entrypoint!`. Accounts are parsed lazily on a strictly needed basis using `ctx.next_account_unchecked()` coupled with `borrowed_data_unchecked()` and raw pointer operations. This strictly avoids large stack allocations (like parsing accounts into arrays) and completely bypasses the standard BPF account serialization overhead.

### 2. Sysvar Optimization via Memory Reads
Standard Solana programs use `Clock::get()?.unix_timestamp`, which triggers an expensive `sol_get_clock_sysvar` syscall (~100 CUs). Instead, we pass the `SysvarClock` account directly as an instruction dependency and read the timestamp straight from memory:
```rust
let clock_ts = unsafe {
    let clock_data = clock.try_borrow()?;
    *(clock_data.as_ptr().add(32) as *const i64)
};
```

### 3. Precomputed Rent Configuration
Syscalls to `Rent::get()` have been completely eliminated. Minimum rent balances for the `Fundraiser` (91 bytes) and `Contributor` (9 bytes) accounts are precomputed securely based on the formula `3480 * 2 * (128 + data_len)` saving significant syscall overhead on initialization instructions.

### 4. P-Token Integration (`pinocchio-token`)
We utilize the `pinocchio_token` library rather than standard SPL token CPIs. This avoids bulky instruction wrappers and minimizes the CPI payload, translating to significantly less CU burn on target token operations (Minting, Transferring, etc).

### 5. Custom 1-byte Discriminators & No-Borsh
Heavy serialization frameworks like Borsh were bypassed. This program utilizes tight 1-byte discriminators for instruction routing. Instruction payloads extract their precise types directly from the byte array (e.g. `amount.to_le_bytes()`). Error logs are reduced to strict `u32` constant code maps (e.g. `0x100`) rather than expensive string log invocations.

---

##  Security Practices

While pushing for maximal performance, all critical security mechanisms are fully intact and guarded against:

1. **Vault Ownership Verification:** Validates that the vault is strictly owned by `pinocchio_token::ID` and directly matches the mint to raise. Protects against fake balance injections and vault-drain attacks.
2. **Strict Signer & Seed Isolation:** Prevents PDA seed collisions by ensuring that `maker` constraints and `bump` constraints match precisely with the initialized Fundraiser configurations.
3. **Overflow Protection:** Raw math implementations strictly use `checked_add` and limit checks to prevent malicious value rolling on contributions.
4. **Direct PDA Mutability Guarantees:** Zero-copy mutable casting directly ensures that account mutations cannot affect unowned data or overlap state contexts.

---

##  Build & Test

### Dependencies
Ensure you have the latest Rust toolchain and Solana CLI versions compatible with `p-token` Devnet iterations.

### Testing
Because of the heavy reliance on raw CPIs and the lack of standard SPL wrappers, it is highly recommended to run tests directly against a test validator instance running with the p-token feature gates active. 

```sh
# Run the internal program unit/integration tests
cargo test -- --nocapture
```

### Build for Devnet
```sh
cargo build-sbf
```
