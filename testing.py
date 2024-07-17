import os
import subprocess
import time

TOKEN_2022 = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
TOKEN = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"

current_dir = os.getcwd()
subfolder_path = "program/tests/fixtures"
print(f"Current working directory: {current_dir}")
test_path = os.path.join(current_dir, subfolder_path)
print(f"Target for account binaries: {test_path}")

accounts: dict[str, str] = {}


# For auto-enter
def create_keypair(keypair: str):
    subprocess.Popen(
        [
            "solana-keygen",
            "new",
            "--force",
            "--no-bip39-passphrase",
            "--outfile",
            f"{current_dir}/{keypair}.json",
        ],
        stdout=subprocess.DEVNULL,
    )


def get_address(keypair: str) -> str:
    return subprocess.run(
        ["solana", "address", "-k", f"{current_dir}/{keypair}.json"],
        capture_output=True,
        text=True,
    ).stdout.strip()


def create_mint(keypair: str, token_program: int | None) -> str:
    command = [
        "spl-token",
        "create-token",
        "--program-id",
        "",
        "--fee-payer",
        f"{current_dir}/payer.json",
        "--mint-authority",
        f"{current_dir}/payer.json",
    ]

    if token_program == 2022:
        command[3] = TOKEN_2022
    else:
        command[3] = TOKEN

    return (
        subprocess.run(command, capture_output=True, text=True)
        .stdout.strip()
        .split("\n")[0]
        .split()[2]
        .strip()
    )


def create_token_account(
    mint_address: str, owner_address: str, token_program: int | None
) -> str:
    command = [
        "spl-token",
        "create-account",
        "--program-id",
        "",
        "--fee-payer",
        f"{current_dir}/payer.json",
        "--owner",
        owner_address,
        mint_address,
    ]

    if token_program == 2022:
        command[3] = TOKEN_2022
    else:
        command[3] = TOKEN

    return (
        subprocess.run(command, capture_output=True, text=True)
        .stdout.strip()
        .split("\n")[0]
        .split()[2]
        .strip()
    )


def create_ticket(mint_address: str) -> str:
    command = [
        "stuk-wl",
        "--payer",
        f"{current_dir}/payer.json",
        "register",
        mint_address,
    ]
    return (
        subprocess.run(command, capture_output=True, text=True)
        .stdout.strip()
        .split()[1]
        .strip()
    )


def create_whitelist(mint_address: str, wallet_address: str) -> str:
    command = [
        "stuk-wl",
        "--payer",
        f"{current_dir}/payer.json",
        "init",
        mint_address,
        wallet_address,
        "1",
        "10",
        "5",
    ]
    return (
        subprocess.run(command, capture_output=True, text=True)
        .stdout.strip()
        .split("\n")[0]
        .split()[2]
        .strip()
    )


def allow_registration(mint_address: str) -> None:
    command = [
        "stuk-wl",
        "--payer",
        f"{current_dir}/payer.json",
        "allow-register",
        mint_address,
        "true",
    ]
    subprocess.run(command)


def generate_account_binary(bin_name: str, account: str) -> None:
    command = [
        "solana",
        "account",
        account,
        "--output-file",
        f"{test_path}/{account}.bin",
    ]
    subprocess.run(command)


# Generate program id keypair
print("Generating program id keypair...")
create_keypair("test-pid")
program_id = get_address("test-pid")
print(f"{program_id}")

# Replace the program id in entrypoint.rs
print("Replacing program id before compilation")
search_string = "'declare_id!'"
sed_command = f"sed -i '/{search_string}/c\\declare_id!(\"{program_id}\");' {current_dir}/program/src/lib.rs"
os.system(sed_command)

# Generate payer keypair
print("Generating payer keypair")
create_keypair("payer")
wallet_address = get_address("payer")

# Generate mint keypairs
print("Generating mint keypairs...")
create_keypair("mint2022")
create_keypair("mint")
print("Mint keypairs generated")

# Generate whitelist keypair
print("Generating whitelist keypair")
create_keypair("whitelist")

# Start a test validator to retrieve account binaries
print("Starting validator in the background")
validator_process = subprocess.Popen(
    [
        "solana-test-validator",
        "--reset",
        "--mint",
        wallet_address,
    ],
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
    text=True,
)

time.sleep(2)
if validator_process.poll() is None:
    print("Solana test validator is running in the background")
else:
    stdout, stderr = validator_process.communicate()
    print("Failed to start Solana test validator")
    print(f"STDOUT: {stdout}")
    print(f"STDERR: {stderr}")
    exit(1)
print("Validator running")

# Compile and deploy the whitelist program
subprocess.run(
    ["cargo", "build-bpf", "--manifest-path", f"{current_dir}/program/Cargo.toml"]
)
subprocess.run(
    [
        "solana",
        "program",
        "deploy",
        f"{current_dir}/program/target/deploy/stuk_wl.so",
        "--program-id",
        f"{current_dir}/test-pid.json",
        "--fee-payer",
        f"{current_dir}/payer.json",
    ]
)

subprocess.run(
    ["cargo", "build", "--release", "--manifest-path", f"{current_dir}/cli/Cargo.toml"]
)
subprocess.run(["cargo", "install", "--path", f"{current_dir}/cli"])

# Create spl-token (2022)
accounts["mint_2022"] = create_mint("mint2022", 2022)
print(f"Mint address(2022): {accounts["mint_2022"]}")

# Create spl-token
accounts["mint"] = create_mint("mint", None)
print(f"Mint address: {accounts["mint"]}")

# Create token account (2022)
accounts["wallet_token_account_2022"] = create_token_account(
    accounts["mint_2022"], wallet_address, 2022
)
print(f"Wallet token address(2022): {accounts["wallet_token_account_2022"]}")

# Create token account
accounts["wallet_token_account"] = create_token_account(
    accounts["mint"], wallet_address, None
)
print(f"Wallet token address: {accounts["wallet_token_account"]}")

# Create whitelist address for token2022
accounts["whitelist_2022"] = create_whitelist(accounts["mint_2022"], wallet_address)
print(f"Whitelist address(2022): {accounts["whitelist_2022"]}")
accounts["whitelist"] = create_whitelist(accounts["mint"], wallet_address)
print(f"Whitelist address: {accounts["whitelist"]}")

# Create the vault
accounts["vault_2022"] = create_token_account(
    accounts["mint_2022"], accounts["whitelist_2022"], 2022
)
print(f"Vault address(2022): {accounts["vault_2022"]}")

accounts["vault"] = create_token_account(accounts["mint"], accounts["whitelist"], None)
print(f"Vault address: {accounts["vault"]}")

# Enable registration
allow_registration(accounts["mint_2022"])
allow_registration(accounts["mint"])

# Create ticket address
accounts["ticket_account_2022"] = create_ticket(accounts["mint_2022"])
print(f"Ticket address(2022): {accounts["ticket_account_2022"]}")

accounts["ticket_account"] = create_ticket(accounts["mint"])
print(f"Ticket address: {accounts["ticket_account"]}")

# Create ticket token accounts
accounts["ticket_token_account_2022"] = create_token_account(
    accounts["mint_2022"], accounts["ticket_account_2022"], 2022
)
print(f"Ticket token account address(2022): {accounts["ticket_token_account_2022"]}")

accounts["ticket_token_account"] = create_token_account(
    accounts["mint"], accounts["ticket_account"], None
)
print(f"Ticket token account address: {accounts["ticket_token_account"]}")

for account in accounts:
    generate_account_binary(account, accounts[account])
# Get account binaries

# Clean up keypairs
print("Cleaning up keypairs...")
os.remove(f"{current_dir}/mint.json")
os.remove(f"{current_dir}/mint2022.json")
os.remove(f"{current_dir}/test-pid.json")
os.remove(f"{current_dir}/whitelist.json")
os.remove(f"{current_dir}/payer.json")
print("Keypairs removed")

print("Stopping solana-test-validator")
subprocess.run(["pkill", "-f", "-9", "solana-test-validator"])
