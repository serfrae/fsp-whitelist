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

## Setup
Deployment of this program costs approximately 2.4 SOL.

Initialisation of a whitelist costs: SOL.

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

```
wl-stuk init <MINT> <TREASURY> <PRICE> <BUY_LIMIT> [WHITELIST_SIZE] [ALLOW_REGISTRATION] [REGISTRATION_START_TIME] [REGISTRATION_DURATION] [SALE_START_TIME] [SALE_DURATION]
```

```
wl-stuk user add <MINT> <USER>
wl-stuk user remove <MINT> <USER>
```

```
wl-stuk token deposit <MINT> <AMOUNT>
wl-stuck token withdraw 
```

```
wl-stuk amend times <MINT> [REGISTRATION_START_TIME] [REGISTRATION_DURATION] [SALE_START_TIME] [SALE_DURATION]
wl-stuk amend size <MINT> [SIZE]
```

```
wl-stuk start registration <MINT>
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

`<MINT>` is the token you wish to register/purchase, the CLI will compute the whitelist address for you and any other necessary public keys required by the program.

