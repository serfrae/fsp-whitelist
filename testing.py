import os
import subprocess
import time

TOKEN_2022 = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"

current_dir = os.getcwd()
subfolder_path = "program/tests/fixtures"
print(f"Current working directory: {current_dir}")
test_path = os.path.join(current_dir, subfolder_path)
print(f"Target for account binaries: {test_path}")

# Generate program id keypair
print("Generating program id keypair...")
subprocess.run(
    ["solana-keygen", "new", "--force", "--outfile", f"{current_dir}/test-pid.json"]
)
program_id = subprocess.run(
    ["solana", "address", "-k", f"{current_dir}/test-pid.json"],
    capture_output=True,
    text=True,
).stdout.strip()
print(f"{program_id}")
# Replace the program id in entrypoint.rs
print("Replacing program id before compilation")
search_string = "'declare_id!'"
sed_command = f"sed -i '/{search_string}/c\\declare_id!(\"{program_id}\");' {current_dir}/program/src/lib.rs"
os.system(sed_command)


# Generate mint keypairs
print("Generating mint keypairs...")
subprocess.run(
    ["solana-keygen", "new", "--force", "--outfile", f"{current_dir}/mint2022.json"],
)
subprocess.run(
    ["solana-keygen", "new", "--force", "--outfile", f"{current_dir}/mint.json"],
)
print("Mint keypairs generated")

# Generate whitelist keypair
print("Generating whitelist keypair")
subprocess.run(
    ["solana-keygen", "new", "--force", "--outfile", f"{current_dir}/whitelist.json"],
)

# Start a test validator to retrieve account binaries
print("Starting validator in the background")
validator_process = subprocess.Popen(
    ["solana-test-validator", "--reset"],
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
    text=True,
)

time.sleep(5)
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
    ]
)

subprocess.run(
    ["cargo", "build", "--release", "--manifest-path", f"{current_dir}/cli/Cargo.toml"]
)
subprocess.run(["cargo", "install", "--path", f"{current_dir}/cli"])

# Create spl-token (2022)
subprocess.run(
    [
        "spl-token",
        "--program-id",
        TOKEN_2022,
        "create-token",
        f"{current_dir}/mint2022.json",
    ]
)

result = subprocess.run(
    ["solana", "address", "-k", f"{current_dir}/mint2022.json"],
    capture_output=True,
    text=True,
)
mint_address_2022 = result.stdout.strip()
print(f"Mint address(2022): {mint_address_2022}")
subprocess.run(
    [
        "solana",
        "account",
        mint_address_2022,
        "--output-file",
        f"{test_path}/mint2022.bin",
    ]
)

# Create spl-token
subprocess.run(["spl-token", "create-token", f"{current_dir}/mint.json"])
result = subprocess.run(
    ["solana", "address", "-k", f"{current_dir}/mint.json"],
    capture_output=True,
    text=True,
)
mint_address = result.stdout.strip()
print(f"Mint address: {mint_address}")
subprocess.run(
    ["solana", "account", mint_address, "--output-file", f"{test_path}/mint.bin"]
)

# Create token account (2022)
result = subprocess.run(
    ["spl-token", "create-account", "--program-id", TOKEN_2022, mint_address_2022],
    capture_output=True,
    text=True,
)
token_address_2022 = result.stdout.strip().split("\n")[0].split()[2].strip()
print(f"Wallet token address(2022): {token_address_2022}")
subprocess.run(
    [
        "solana",
        "account",
        token_address_2022,
        "--output-file",
        f"{test_path}/wallet_token_account2022.bin",
    ]
)

# Create token account
token_address = (
    subprocess.run(
        ["spl-token", "create-account", mint_address], capture_output=True, text=True
    )
    .stdout.strip()
    .split("\n")[0]
    .split()[2]
    .strip()
)
print(f"Wallet token address: {token_address}")
subprocess.run(
    [
        "solana",
        "account",
        token_address,
        "--output-file",
        f"{test_path}/wallet_token_account.bin",
    ]
)

wallet_address = subprocess.run(
    ["solana", "address"], capture_output=True, text=True
).stdout.strip()
print(f"Wallet address: {wallet_address}")
# Create whitelist address for token2022
whitelist_address_2022 = (
    subprocess.run(
        ["stuk-wl", "init", mint_address_2022, wallet_address, "1", "10", "5"],
        capture_output=True,
        text=True,
    )
    .stdout.strip()
    .split("\n")[0]
    .split()[2]
    .strip()
)

print(f"Whitelist address(2022): {whitelist_address_2022}")
subprocess.run(
    [
        "solana",
        "account",
        whitelist_address_2022,
        "--output-file",
        f"{test_path}/whitelist2022.bin",
    ]
)

# Create whitelist address
whitelist_address = (
    subprocess.run(
        ["stuk-wl", "init", mint_address, wallet_address, "1", "10", "5"],
        capture_output=True,
        text=True,
    )
    .stdout.strip()
    .split("\n")[0]
    .split()[2]
    .strip()
)

print(f"Whitelist address: {whitelist_address}")
subprocess.run(
    [
        "solana",
        "account",
        whitelist_address,
        "--output-file",
        f"{test_path}/whitelist.bin",
    ]
)
# Enable registration
subprocess.run(["stuk-wl", "allow-register", mint_address_2022, "true"])
subprocess.run(["stuk-wl", "allow-register", mint_address, "true"])

# Create ticket address (2022)
ticket_address_2022 = (
    subprocess.run(
        ["stuk-wl", "register", mint_address_2022], capture_output=True, text=True
    )
    .stdout.strip()
    .split()[1]
    .strip()
)
print(f"Ticket address(2022): {ticket_address_2022}")
subprocess.run(
    [
        "solana",
        "account",
        ticket_address_2022,
        "--output-file",
        f"{test_path}/ticket2022.bin",
    ]
)

# Create ticket address
ticket_address = (
    subprocess.run(
        ["stuk-wl", "register", mint_address], capture_output=True, text=True
    )
    .stdout.strip()
    .split()[1]
    .strip()
)
print(f"Ticket address: {ticket_address}")
subprocess.run(
    ["solana", "account", ticket_address, "--output-file", f"{test_path}/ticket.bin"]
)

# Clean up keypairs
print("Cleaning up keypairs...")
os.remove(f"{current_dir}/mint.json")
os.remove(f"{current_dir}/mint2022.json")
os.remove(f"{current_dir}/test-pid.json")
os.remove(f"{current_dir}/whitelist.json")
print("Keypairs removed")

print("Stopping solana-test-validator")
subprocess.run(["pkill", "-f", "-9", "solana-test-validator"])
