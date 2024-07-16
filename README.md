# Whitelist-Gated Token Sale

This program and it's associated client and blink permit users to register for a token presale.

UPDATE: It occurred to me that deploying without the program's keypair would result in errors when trying to invoke, I have removed `./install.sh` and replaced it with `testing.py` that will setup a test-validator and populate `tests/fixtures` and replace the program id in lib.rs with one generated by the script using `sed`, this script will terminate the test validator after running in an elegant manner (invoking pkill) be wary of this if you have another test-validator running on your system.

## Implementation
This program allows for a whitelist-gated token sale. It supports both spl_token and spl_token_2022 accounts and is intended to
be as feature-rich as possible while enabling a large range of customization options.

The program is also optimised for parallel execution to avoid the typical bottlenecks: when registration begins and the token sale
itself, these bottlenecks are caused by writes to the program's account state which may only occur once per slot. This program
avoids these bottlenecks by enabling the seller to preload tokens into the ticket accounts so that when a token sale begins transfers are made 
from the ticket account to the user's token account and SOL is transferred from the user wallet to the ticket account's wallet, thus avoiding
writes to the same account. The program is able to recognise whether the ticket account has been pre-loaded, if ticket accounts are not pre-loaded the
program will transfer from the token vault instead. A seller can then sweep the accounts after the token sale has completed, all SOL and rent will be 
transferred into the designated treasury account.

A seller may also define whether or not to permit users to register for the whitelist or add them manually by setting the flag `allow_registration`.
When set to false, users will not be able to register, this may also be used to freeze registration for whatever purpose.

A seller can set timestamps to automatically commence registration or token sales, or manually trigger these themselves using the `StartRegistration`
or `StartTokenSale` instructions.

It should be noted that providing a duration for each phase of the sale is optional, but is not recommended for the token sale, as a user will not be able to
withdraw tokens/sol unless all tokens are distributed. There does however, exist a workaround, where a seller may terminate the user's ticket using the `RemoveUser`
instruction to "burn" a ticket and transfer all tokens and sol back to themselves.

A seller may also amend the times and durations, however attempting to amend a time after it's original value has elapsed will throw an error.
A seller may set a whitelist size and the client will check for the number of registered users via a call to the rpc method `get_program_account`, the client will
search for accounts associated with the whitelist. I may implement this search to be multi-threaded in a future release and improve the array traversal, but for
the purpose of this SOW this should be suitable.

## Setup
Deployment of this program costs approximately 2.61 SOL.

Initialisation of a whitelist has a negligible costs.

Please ensure you have enough SOL in your wallet on the respective network (testnet/devnet/mainnet-beta).

To begin please clone this repo by copying:

```git clone https://github.com/serfrae/stuk-whitelist```

After cloning the repo navigate into it:

```cd stuk-whitelist```

To setup the CLI and deploy the program, an installation script has been provided for your convenience,
please ensure to change the wallet addresses to the path of the wallet you wish to use for the program-id and mints.
The script will automatically utilise the wallet found at `$XDG_CONFIG_HOME/solana/id.json` as the authority.

After you have edited the script with your desired addresses/wallets, ensure that it is executable using:

```chmod +x install.sh```

then install by using:

```./install.sh```

at the top-level directory to begin deployment and installation of the CLI. The script will not setup the blinks, as
of this writing they are not tested.

After installation the CLI can be invoked from the command line with:

```stuk-wl```

The CLI will automatically utilise the wallet address at `$XDG_CONFIG_HOME/solana/id.json` when being invoked. As the program itself is quite feature-full and this was developed in two (2) days I have not had time to enable reading from configs or pointing to custom wallet
addresses from within the CLI, although I may implement this at a later date.

## Usage - Seller
`stuk-wl --help` will provide information on each command and subcommand

### Initialisation
```
stuk-wl init <MINT> <TREASURY> <PRICE> <BUY_LIMIT> <WHITELIST_SIZE> <ALLOW_REGISTRATION> [REGISTRATION_START_TIME] [REGISTRATION_END_TIME] [SALE_START_TIME] [SALE_END_TIME]
```
- `MINT`: The public key of the mint for the token that is to be sold, this field is used for all commands, the whitelist address is derived from it.
- `TREASURY`: The target for both SOL and token withdrawals when burning tickets.
- `PRICE`: Price in SOL per token
- `BUY_LIMIT`: Number of tokens a ticket is allowed to purchase.
- `WHITELIST_SIZE`: The size of the whitelist, i.e. how many users can register for the token sale. 
- `ALLOW_REGISTRATION`: (values: `"true" / "yes" / "y", "false" / "no" / "n"`) Permit users to register for the whitelist.
- [optional]`REGISTRATION_START_TIME` (format: YYYY-MM-DD HH:MM:SS): When registration commences, a 0 value will allow immediate registration
    - Requires flag `--registration-start-time`
- [optional]`REGISTRATION_END_TIME` (format: YYYY-MM-DD HH:MM:SS): When registration ends, a 0 value means that registration does not end.
    - Requires flag `--registration-end-time`
- [optional]`SALE_START_TIME` (format: YYYY-MM-DD HH:MM:SS): When the token sale starts, a 0 value means that the sale immediately starts.
    - Requires flag `--sale-start-time`
- [optional]`SALE_END_TIME` (format: YYYY-MM-DD HH:MM:SS): When the token sale ends, a 0 value means that registration does not end. (WARNING: NOT RECOMMENDED).
    - Requires flag `--sale-end-time`

### User Management
```
stuk-wl user add <MINT> <USER>
stuk-wl user remove <MINT> <USER>
```
- `add`: Add a user to the whitelist associated with the provided mint where `MINT` is the mint address of the token being sold and `USER` is the user's wallet address.
- `remove`: Remove a user from the whitelist and claim rent where `MINT` is the mint address of the token being sold and `USER` is the user's wallet address.

### Deposit
```
stuk-wl deposit <MINT> <AMOUNT>
```
- Deposits tokens into the whitelist vault, where `MINT` is the mint address of the token being sold and `AMOUNT` is the amount of tokens to transfer into the vault.

### Withdraw
```
stuk-wl withdraw <MINT>
```
- Withdraws tokens from the vault, tokens may not be withdrawn from the vault after the token sale begins. `MINT` is the mint address of the token being sold.

### Amend
#### Amend Whitelist Size
```
stuk-wl amend size <MINT> <SIZE>
```
- Amend the whitelist size allowing for more users to register for the token sale. Where `MINT` is the mint address of the token being sold and `SIZE` is the number of users permitted to register for the token sale.

#### Amend Times
```
stuk-wl amend times [REGISTRATION_START_TIME] [REGISTRATION_END_TIME] [SALE_START_TIME] [SALE_END_TIME]
```
Note: Each argument must be provided with a flag.

### Start
#### Start Registration
```
stuk-wl start registration <MINT>
```
- Commences registration upon successful transaction, will also set the `allow_registration` field to `true` and the `registration_start_timestamp` to the current unix timestamp in the whitelist account's state. `MINT` is the mint address of the token for sale.

#### Start Token Sale
```
stuk-wl start sale <MINT>
```
- Commences the token sale upon successful transaction, will also set `sale_start_timestamp` to the current unix timestamp in the whitelist's account state. `MINT` is the mint address of the token for sale. 

### Allow Registration
```
stuk-wl allow-register <MINT> <ALLOW>
```
- (`"true" / "yes" / "y", "false / "no" / "n"`) Sets the `allow_registration` flag in the whitelist's account state. `MINT` is the mint address of the token for sale and `ALLOW` is one of the values provided where `"true"`, `"yes"` and `"y"` all enable registration while `"false"`, `"no"` and `"n"` disables registration. This may also be used to freeze currently ongoing registrations but may cause errors.

### Burn Tickets
#### Burn a single ticket
```
stuk-wl burn single <MINT> <USER>

```
- Burns a single ticket and retrieves the tokens and SOL associated with the ticket. Tokens and SOL are sent to the treasury address defined in the whitelist's state. `MINT` is the mint address of the token for sale, `USER` is the wallet address of the user who purchased the ticket. 

#### Burn all tickets
```
stuk-wl burn bulk <MINT>
```
- Burns all tickets associated with a whitelist and retrieves the tokens and SOL associated with those tickets. Tokens and SOL are sent to the treasury address defined in the whitelist's state. `MINT` is the mint address for the token for sale.

### Terminate Whitelist
```
stuk-wl close <MINT> [RECIPIENT]
```
- Terminates the whitelist and closes all associated accounts reclaiming and tokens and rent to the designated recipient, if no recipient is provided, tokens and rent are transferred to the authority / caller. `MINT` is the mint address of the token for sale `RECIPIENT` takes a flag `---recipient` to define the address of the account to which rent and tokens should be sent. A whitelist may not be terminated until the token sale has ended.

### Info
#### Whitelist Info
```
stuk-wl info whitelist <MINT> 
```
- Retrieves information about the whitelist, mostly the parameters that are set in the whitelist's state. `MINT` is the mint address of the token for sale.

#### User Info
```
stuk-wl info user <MINT> <USER>
```
- Retrieves information about a user's ticket. `MINT` is the mint address of the token for sale, `USER` is the wallet address of the user you wish to retrieve ticket information about. An error means there is no ticket associated with the provided user wallet address.

## Usage - Buyer 
There are only four (4) commands relevant to a whitelist subscriber/buyer in the CLI these being:
```
1. stuk-wl register <MINT>
2. stuk-wl unregister <MINT>
3. stuk-wl token buy <MINT> <AMOUNT>
3. stukw-wl info whitelist/user <PUBKEY> <MINT>
```

### Register
```
stuk-wl register <MINT>
```
- Register for the token sale, creating a ticket. `MINT` is the mint address of the token for sale, information about a created ticket can be retrieved using `stuk-wl info user` - see below.

### Unregister
```
stuk-wl unregister <MINT>
```
- Unregister for the token sale, burning a ticket. Rent is reclaimed and transferred back to the payer, if the user is the payer, the to the user, else it is transferred to the whitelist's authority. `MINT` is the mint address of the token for sale.

### Buy
``` 
stuk-wl buy <MINT> <AMOUNT>
```
- Buy tokens, users may only buy tokens if they posses a ticket i.e. are registered to the whitelist, a user may not purchase more tickets than the buy limit / their ticket allowance, doing so will result in transaction failure. `MINT` is the mint address of the token being sold, `AMOUNT` is the amount of tokens a user wishes to purchase.

### Info
#### Whitelist Info
```
stuk-wl info whitelist <MINT> 
```
- Retrieves information about the whitelist, mostly the parameters that are set in the whitelist's state. `MINT` is the mint address of the token for sale.

#### User Info
```
stuk-wl info user <MINT> <USER>
```
- Retrieves information about a user's ticket. `MINT` is the mint address of the token for sale, `USER` is the wallet address of the user you wish to retrieve ticket information about. An error means there is no ticket associated with the provided user wallet address.

