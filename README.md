<div align="center">
  <h1> Solana Turbin3 Accelerated Course</h1>
  <p><strong>Developer:</strong> Pankaj | <strong>GitHub:</strong> <a href="https://github.com/0x-pankaj">@0x-pankaj</a></p>
  <p>
    <a href="https://github.com/solana-turbin3/Q1_acc_pankaj"><img src="https://img.shields.io/badge/Repository-solana--turbin3%2FQ1__acc__pankaj-blue?style=for-the-badge&logo=github" alt="Turbin3 Repo" /></a>
    <a href="https://solana.com/"><img src="https://img.shields.io/badge/Solana-14F195?style=for-the-badge&logo=solana&logoColor=white" alt="Solana" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" /></a>
  </p>
</div>

Welcome to my portfolio of projects, smart contract implementations, and assignments completed during the **Solana Turbin3 Accelerated Course (Q1)**. This repository contains all my weekly submissions and weekend tasks, nicely organized below.

---

##  Course Work

###  Week 1 

#### 1. Token Extensions: Whitelist Transfer Hook
- **Description:** 
  - **Part 1:** Created a Program Derived Address (PDA) account for each whitelisted address, instead of pushing them into a standard vector.
  - **Part 2:** Created the token directly within the program, using the token extension arguments.
- **Local Source:** [`./whitelist-transfer-hook`](./whitelist-transfer-hook)
- **External Links:** 
  - [Personal Repo Submission](https://github.com/0x-pankaj/accelerated-solanaturbine/tree/main/whitelist-transfer-hook)
  - [Turbin3 Repo Submission](https://github.com/solana-turbin3/Q1_acc_pankaj/tree/main/whitelist-transfer-hook)

#### 2. LiteSVM Testing 
- **Description:** 
  - **Part 1:** Completed the tests for the `take` and `refund` instructions for an Escrow program.
  - **Part 2:** Modified the program to introduce a required condition for the `take` instruction to happen at least 5 days after the `make` instruction. Added robust tests accordingly.
- **Starter Repo:** [ASCorreia/escrow-litesvm](https://github.com/ASCorreia/escrow-litesvm)
- **Local Source:** [`./escrow-litesvm`](./escrow-litesvm)
- **External Link:** [Turbin3 Repo Submission](https://github.com/solana-turbin3/Q1_acc_pankaj/tree/main/escrow-litesvm)

#### 3. Weekend Task: Transfer Hook Vault
- **Description:** Created a Transfer Hook Vault where *only* whitelisted users can interact with the vault.
  - Uses a single vault and mints the token directly in the program.
  - Hook interactions apply to both depositing and removing funds.
  - Initially implemented a vector for the whitelist (`Pubkey` and amount) and progressed to testing PDA-based solutions.
  - Everything is fully tested with LiteSVM.
  - Includes additional token extensions.
- **Local Source:** [`./transfer-hook-vault`](./transfer-hook-vault)
- **External Link:** [Turbin3 Repo Submission](https://github.com/solana-turbin3/Q1_acc_pankaj/tree/main/transfer-hook-vault)

---

### Week 2

#### 1. VRF Implementation (Magicblock Ephemeral Rollups)
- **Description:** Implemented Verifiable Random Function (VRF) capabilities within the example repo to update user state data.
  - **Task 1:** Implemented VRF outside the Ephemeral Rollup (ER).
  - **Task 2:** Implemented VRF seamlessly inside the Ephemeral Rollup (ER).
- **Starter Repo:** [ASCorreia/magicblock-er-example](https://github.com/ASCorreia/magicblock-er-example)
- **Local Source:** [`./magicblock-er-example`](./magicblock-er-example)
- **External Link:** [Turbin3 Repo Submission](https://github.com/solana-turbin3/Q1_acc_pankaj/tree/main/magicblock-er-example)

#### 2. Tuktuk Scheduler Implementation
- **Description:** Implemented the scheduler and automated cron job capabilities in one of the previous challenges. Applied Tuktuk workflows to automatically handle scheduled actions using Cross-Program Invocations (CPI).
- **Helper Repo:** [ASCorreia/tuktuk-counter](https://github.com/ASCorreia/tuktuk-counter)
- **Local Source:** [`./escrow-litesvm-tuktuk`](./escrow-litesvm-tuktuk)
- **External Link:** [Turbin3 Repo Submission](https://github.com/solana-turbin3/Q1_acc_pankaj/tree/main/escrow-litesvm-tuktuk)

#### 3. Weekend Task: Tuktuk GPT Oracle
- **Description:** Scheduled the Solana GPT Oracle (provided by MagicBlock) seamlessly with Tuktuk. The program successfully and correctly routes requests and captures the parsed response back from the GPT Oracle.
- **Local Source:** [`./gpt-tuktuk`](./gpt-tuktuk)
- **External Link:** [Turbin3 Repo Submission](https://github.com/solana-turbin3/Q1_acc_pankaj/tree/main/gpt-tuktuk)

---

### Week 3

#### 1. Week 3 Subtask
- **Description:** Generic-storage-challenge [borsh, wincode, serde] && persistant todo-cli with borsh
- **Local Source:** [`./week-3`](./week-3)
- **External Link:** [Turbin3 Repo Submission](https://github.com/solana-turbin3/Q1_acc_pankaj/tree/main/week-3)

---

<div align="center">
  <i>Built with 🦀 Rust &  Solana</i>
</div>
