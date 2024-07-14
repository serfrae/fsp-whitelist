# Whitelist-Gated Token Sale

This program and it's associated client and blink permit users to register for a token presale.

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

After you have edited the script with your desired addresses/wallets use:

```./install.sh```

At the top-level directory to begin deployment and installation of the CLI, the script will not setup the blinks, as
of this writing they are not tested.

After installation the CLI can be invoked from the command line with:

```stuk-wl```

The CLI will automatically utilise this same address when being invoked. As the program itself is quite feature-full
and this was developed in two (2) days I have not had time to enable reading from configs or pointing to custom wallet
addresses from within the CLI, although I may implement this at a later date.

## Usage - Seller

## Usage - User
There are only four (4) commands relevant to a whitelist subscriber in the CLI these being:
```
1. stuk-wl register <MINT>
2. stuk-wl unregister <MINT>
3. stuk-wl buy <MINT> <AMOUNT>
3. stukw-wl info whitelist/user <PUBKEY> <MINT>
```

`<MINT>` is the token you wish to register/purchase, the CLI will compute the whitelist address for you and any other necessary public keys required by the program.

