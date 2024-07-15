# Whitelist-Gated Token Sale

This program and it's associated client and blink permit users to register for a token presale.

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

At the top-level directory to begin deployment and installation of the CLI, the script will not setup the blinks, as
of this writing they are not tested.

After installation the CLI can be invoked from the command line with:

```stuk-wl```

The CLI will automatically utilise this same address when being invoked. As the program itself is quite feature-full
and this was developed in two (2) days I have not had time to enable reading from configs or pointing to custom wallet
addresses from within the CLI, although I may implement this at a later date.

Because of some trauma I got from trying to implement support for both versions of the token program, both the CLI and the program will check the mint for the owner and use that to pass the correct token program.

## Usage - Seller
`wl-stuk --help` will provide information on each command and subcommand

### Initialisation
```
wl-stuk init <MINT> <TREASURY> <PRICE> <BUY_LIMIT> <WHITELIST_SIZE> <ALLOW_REGISTRATION> [REGISTRATION_START_TIME] [REGISTRATION_END_TIME] [SALE_START_TIME] [SALE_END_TIME]
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
wl-stuk user add <MINT> <USER>
wl-stuk user remove <MINT> <USER>
```
- `add`: Add a user to the whitelist associated with the provided mint where `MINT` is the mint address of the token being sold and `USER` is the user's wallet address.
- `remove`: Remove a user from the whitelist and claim rent where `MINT` is the mint address of the token being sold and `USER` is the user's wallet address.

### Deposit
```
wl-stuk deposit <MINT> <AMOUNT>
```
- Deposits tokens into the whitelist vault, where `MINT` is the mint address of the token being sold and `AMOUNT` is the amount of tokens to transfer into the vault.

### Withdraw
```
wl-stuk withdraw <MINT>
```
- Withdraws tokens from the vault, tokens may not be withdrawn from the vault after the token sale begins. `MINT` is the mint address of the token being sold.


### Amend
#### Amend Whitelist Size
```
wl-stuk amend size <MINT> <SIZE>
```
- Amend the whitelist size allowing for more users to register for the token sale. Where `MINT` is the mint address of the token being sold and `SIZE` is the number of users permitted to register for the token sale.

#### Amend Times
```
wl-stuk amend times [REGISTRATION_START_TIME] [REGISTRATION_END_TIME] [SALE_START_TIME] [SALE_END_TIME]
```
Note: Each argument must be provided with a flag.

### Start
#### Start Registration
```
wl-stuk start registration <MINT>
```

#### Start Token Sale
```
wl-stuk start sale <MINT>
```
```
wl-stuk registration <MINT> <ALLOW>
```

```
wl-stuk close <MINT> [RECIPIENT]
```

```
wl-stuk info whitelist <MINT>
wl-stuk info user <MINT> <USER>
```

## Usage - User
There are only four (4) commands relevant to a whitelist subscriber in the CLI these being:
```
1. stuk-wl register <MINT>
2. stuk-wl unregister <MINT>
3. stuk-wl token buy <MINT> <AMOUNT>
3. stukw-wl info whitelist/user <PUBKEY> <MINT>
```

### Register

### Unregister

### Buy
```
wl-stuk buy <MINT> <AMOUNT>
```
- Buy tokens, users may only buy tokens if they posses a ticket i.e. are registered to the whitelist, a user may not purchase more tickets than the buy limit / their ticket allowance, doing so will result in transaction failure. `MINT` is the mint address of the token being sold, `AMOUNT` is the amount of tokens a user wishes to purchase.

### Info
```
stuk-wl info whitelist <MINT>
stuk-wl info user <MINT> <USER_PUBKEY>
```

