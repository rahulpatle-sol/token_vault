# 🔒 Token Vault (Solana Devnet)

A **time-locked SPL Token Vault** built using **Anchor** on the **Solana Devnet**.  
This vault allows users to **deposit**, **lock**, and **withdraw** tokens based on time restrictions.

---

## 🚀 Program Details

| Field | Value |
|-------|--------|
| **Program ID** | `8WijvK9GJ5q1KSP1o1xuH4J1qw9VHie47riZecc9zmBS` |
| **Network** | Devnet |
| **Authority (Wallet Pubkey)** | *Your Solana Keypair Pubkey* |
| **Status** | ✅ Successfully Deployed |
signature hash :4sHtAzv9HKy5T6hao6Ct2FTZVQbPiJRJrpJwQAgZ4GiHCUHsXF2fw5uPMRFsD8vUzdN5Tk6nvTRqJpdYfxhE8vuk
account main:4sHtAzv9HKy5T6hao6Ct2FTZVQbPiJRJrpJwQAgZ4GiHCUHsXF2fw5uPMRFsD8vUzdN5Tk6nvTRqJpdYfxhE8vuk
new address :EfSEJALzrT3gVWMkNZCPjJ5MkF7v7tWWPpNfGMwhMVzD
---
<img width="1139" height="401" alt="image" src="https://github.com/user-attachments/assets/6dc4b6a7-f87e-4aee-ba45-4ebaa17e98d3" />
<img width="1123" height="476" alt="image" src="https://github.com/user-attachments/assets/4fb53aa5-eec5-4b1f-8319-0abb9c1fe6a1" />


## 🧠 Features

- Deposit SPL tokens into a secure vault  
- Lock tokens for a specific duration  
- Withdraw tokens only after the lock period ends  
- Prevents premature withdrawals using time validation  

---

## 🛠 Setup & Commands

Before running, make sure your Solana CLI is set to **Devnet**:
```bash
solana config set --url devnet
| Command                     | Description                                |
| --------------------------- | ------------------------------------------ |
| `anchor build`              | Compiles the Rust program                  |
| `anchor deploy`             | Deploys or updates the program on Devnet   |
| `anchor test --skip-deploy` | Runs tests against the live Devnet program |
| Account                 | Seeds                         | Purpose                               |
| ----------------------- | ----------------------------- | ------------------------------------- |
| **Vault PDA**           | `["vault", payer_pubkey]`     | Stores vault data and lock state      |
| **Vault Authority PDA** | `["authority", vault_pubkey]` | Signs withdrawals, owns token account |
| Type                            | Description                                |
| ------------------------------- | ------------------------------------------ |
| **Program Account (Vault PDA)** | Created during `initialize_vault`          |
| **Vault Token Account**         | SPL token account holding deposited tokens |
| Instruction        | Description                        | Restriction                               |
| ------------------ | ---------------------------------- | ----------------------------------------- |
| `initialize_vault` | Initializes the vault and accounts | None                                      |
| `deposit`          | Deposits SPL tokens into the vault | None                                      |
| `lock_vault`       | Locks the vault for a duration     | Must set `unlock_timestamp` in the future |
| `withdraw`         | Withdraws tokens from vault        | Fails if still locked                     |
| `unlock_vault`     | Unlocks vault manually             | Fails if before unlock time               |
anchor test --skip-deploy
📄 License

MIT © 2025 — Token Vault on Solana
