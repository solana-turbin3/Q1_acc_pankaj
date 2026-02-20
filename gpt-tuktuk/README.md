#  GPT-Tuktuk — Solana GPT Oracle with Tuktuk Scheduler

A Solana program that schedules GPT Oracle requests using [Tuktuk](https://github.com/helium/tuktuk) (a decentralized task scheduler) and receives LLM responses through [MagicBlock's Solana GPT Oracle](https://crates.io/crates/solana-gpt-oracle).

## Architecture

```
┌──────────┐    schedule_request     ┌──────────────┐
│  Client  │ ─────────────────────▶  │  gpt-tuktuk  │
└──────────┘                         │  (Anchor)    │
                                     └──────┬───────┘
                                            │
             ┌──────────────────────────────┤
             │                              │
             ▼                              ▼
    ┌─────────────────┐          ┌────────────────────┐
    │  Tuktuk Queue   │          │   GPT Oracle CPI   │
    │  (task scheduler)│         │  interact_with_llm  │
    └─────────────────┘          └────────┬───────────┘
                                          │
                                          ▼
                                ┌───────────────────────┐
                                │  MagicBlock Ephemeral  │
                                │  Rollup (delegation)   │
                                └────────┬──────────────┘
                                         │
                                         ▼
                                ┌───────────────────┐
                                │  Off-chain Oracle  │
                                │  Agent (LLM call)  │
                                └────────┬──────────┘
                                         │ callback_from_llm
                                         ▼
                                ┌───────────────────┐
                                │  consume_result    │
                                │  (writes response) │
                                └───────────────────┘
```

## On-Chain Flow

| Step | Instruction | What Happens |
|------|------------|--------------|
| 1 | `schedule_request` | Creates `GptRequest` PDA, queues a Tuktuk task with a delay |
| 2 | `execute_request` | CPIs into Oracle's `interact_with_llm` with prompt + callback info |
| 3 | `delegate_interaction` | Delegates the Interaction account to MagicBlock's ephemeral rollup |
| 4 | *(off-chain)* | Oracle agent processes the prompt via LLM |
| 5 | `callback_from_llm` | Oracle CPIs back into our `consume_result` with the response |
| 6 | `consume_result` | Writes the LLM response to `GptRequest` and sets `is_completed = true` |

## Program Accounts

### `GptRequest` PDA
- **Seeds:** `["gpt_request", user, task_id]`
- **Fields:** `task_id`, `prompt`, `result`, `is_completed`, `context_account`, `bump`

### `queue_authority` PDA
- **Seeds:** `["queue_authority"]`
- Used as the Tuktuk queue authority for scheduling tasks via CPI

## Key Program IDs

| Program | ID |
|---------|-----|
| **gpt-tuktuk** | `3LWmo92AxMjU5tLjfneoqCYVMQSoA6teNeYhSiQojpSG` |
| **GPT Oracle** | `LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab` |
| **Tuktuk** | `tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA` |
| **Delegation** | `DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh` |

## Prerequisites

- [Anchor CLI](https://www.anchor-lang.com/docs/installation) v0.31.1
- [Solana CLI](https://docs.solanalabs.com/cli/install) with a devnet wallet funded with SOL
- Node.js & Yarn

## Setup

### 1. Install Dependencies

```bash
yarn install
```

### 2. Build the Program

```bash
anchor build
```

### 3. Deploy to Devnet

```bash
anchor deploy --provider.cluster devnet
```

### 4. Setup Devnet Accounts (One-Time)

This creates the Tuktuk Task Queue and adds our program's `queue_authority` PDA as an authorized queue authority:

```bash
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
ANCHOR_WALLET=~/.config/solana/id.json \
npx ts-node scripts/setup-devnet.ts
```

**What it does:**
- Creates a Tuktuk Task Queue (`gpt-queue-auth2`)
- Adds the program's `queue_authority` PDA as an authority on the queue
- Attempts to create a GPT Oracle context account

### 5. Create Oracle Context (One-Time per Fresh Run)

Each Oracle context gives you a unique Interaction PDA. After delegation, the Interaction account ownership changes, so you need a new context for subsequent runs:

```bash
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
ANCHOR_WALLET=~/.config/solana/id.json \
npx ts-node scripts/create-context.ts
```

**Output:**
```
Current counter: 84
New context account: JADebfDmBbBoq8cGGJ1MCF5o7dYKPH6edxE28g2kq7nk
Interaction PDA: CsWdJ6encpMruzVxFj4ZVgQxL2UWnzH7WC42ngTTb8ST
```

> ⚠️ **Update the `ORACLE_CONTEXT_ACCOUNT` constant in `tests/gpt-tuktuk.ts`** with the new context address after each fresh context creation.

## Running Tests

Tests run directly against **devnet** (not localnet), since the GPT Oracle and Tuktuk are deployed there:

```bash
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
ANCHOR_WALLET=~/.config/solana/id.json \
yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts
```

### Expected Output

```
  gpt-tuktuk
    ✔ Is initialized!
    ✔ Schedules a GPT request via Tuktuk
    ✔ Executes a GPT request (calls GPT Oracle CPI)
    ✔ Delegates interaction to MagicBlock ephemeral rollup
    ✔ Polls for GPT response (devnet integration)

  5 passing
```

### Re-Running Tests

Each run requires:
1. **Bump `taskId`** in `tests/gpt-tuktuk.ts` (e.g., `3` → `4`) — Tuktuk task accounts persist on-chain
2. **New Oracle context** if the previous Interaction was already delegated — run `scripts/create-context.ts` and update the constant

## Project Structure

```
gpt-tuktuk/
├── programs/gpt-tuktuk/src/
│   ├── lib.rs                    # Program entry point
│   ├── state.rs                  # GptRequest account definition
│   └── instructions/
│       ├── initialize.rs         # No-op initialization
│       ├── schedule_request.rs   # Creates PDA + queues Tuktuk task
│       ├── execute_request.rs    # CPIs to Oracle's interact_with_llm
│       └── consume_result.rs     # Receives Oracle callback with LLM response
├── tests/
│   └── gpt-tuktuk.ts            # End-to-end devnet tests
├── scripts/
│   ├── setup-devnet.ts           # One-time Tuktuk queue setup
│   └── create-context.ts         # Creates fresh Oracle context accounts
├── gpt-oracle.json               # GPT Oracle IDL
└── Anchor.toml                   # Anchor configuration
```

## How It Works

### Scheduling (schedule_request)
The client sends a prompt and delay. The program:
1. Initializes a `GptRequest` PDA with the prompt
2. Compiles an `execute_request` instruction
3. Queues it as a Tuktuk task with the specified delay

### Execution (execute_request)
When triggered (manually or by Tuktuk crank):
1. Reads the prompt from `GptRequest`
2. Builds the Oracle's `interact_with_llm` instruction data (prompt, callback program ID, callback discriminator, account metas)
3. CPIs into the GPT Oracle, creating an `Interaction` account

### Delegation (delegate_interaction)
The client calls the Oracle's `delegate_interaction` to:
1. Hand the Interaction account to MagicBlock's ephemeral rollup
2. The Oracle's off-chain agent monitors delegated interactions

### Callback (consume_result)
When the Oracle agent processes the LLM request:
1. It calls `callback_from_llm` on the Oracle program
2. The Oracle CPIs into our `consume_result` with the response
3. `GptRequest.result` is populated and `is_completed` is set to `true`

## Notes

- The Oracle's off-chain agent (`A1ooMmN1fz6LbEFrjh6GukFS2ZeRYFzdyFjeafyyS7Ca`) must be actively running on devnet for callbacks to arrive
- The Tuktuk queue has a capacity of 10 tasks — task IDs must be 0-9
- After delegating an Interaction, its ownership transfers to the Delegation Program — you cannot reuse it for a new `interact_with_llm` call without undelegating first
