import { PublicKey } from "@solana/web3.js";
import { Schema, serialize } from "borsh";
import { Numberu64, Numberi64 } from "./numbers";


class WhitelistSchema {
    bump: number;
    authority: PublicKey;
    vault: PublicKey;
    treasury: PublicKey;
    mint: PublicKey;
    tokenPrice: Numberu64;
    buyLimit: Numberu64;
    deposited: Numberu64;
    whitelistSize: Numberu64;
    allowRegistration: boolean;
    registrationTimestamp: Numberi64;
    registrationDuration: Numberi64;
    saleTimestamp: Numberi64;
    saleDuration: Numberi64;

    constructor(fields: {
        bump: number,
        authority: PublicKey,
        vault: PublicKey,
        treasury: PublicKey,
        mint: PublicKey,
        tokenPrice: Numberu64,
        buyLimit: Numberu64,
        deposited: Numberu64
        whitelistSize: Numberu64,
        allowRegistration: boolean,
        registrationTimestamp: Numberi64
        registrationDuration: Numberi64,
        saleTimestamp: Numberi64,
        saleDuration: Numberi64,
    }) {
        this.bump = fields.bump;
        this.authority = fields.authority;
        this.vault = fields.vault;
        this.treasury = fields.treasury;
        this.mint = fields.mint;
        this.tokenPrice = fields.tokenPrice;
        this.buyLimit = fields.buyLimit;
        this.allowRegistration = fields.allowRegistration;
        this.registrationTimestamp = fields.registrationTimestamp;
        this.registrationDuration = fields.registrationDuration;
        this.saleTimestamp = fields.saleTimestamp;
        this.saleDuration = fields.saleDuration;
    }

    static schema: Schema = {
        struct: {
            bump: "u8",
            authority: { array: { type: "u8", len: 32 } },
            vault: { array: { type: "u8", len: 32 } },
            treasury: { array: { type: "u8", len: 32 } },
            mint: { array: { type: "u8", len: 32 } },
            tokenPrice: "u64",
            buyLimit: "u64",
            deposited: "u64",
            whitelistSize: "u64",
            allowRegistration: "bool",
            registrationTimestamp: "i64",
            registrationDuration: "i64",
            saleTimestamp: "i64",
            saleDuration: "i64",
        }
    };

    serialize(): Uint8Array {
        return serialize(WhitelistSchema.schema, this);
    }
}

class TicketSchema {
    bump: number;
    whitelist: PublicKey;
    owner: PublicKey;
    payer: PublicKey;
    allowance: PublicKey;
    amountBought: PublicKey;

    constructor(fields: {
        bump: number,
        whitelist: PublicKey,
        owner: PublicKey,
        payer: PublicKey,
        allowance: PublicKey,
        amountBought: PublicKey,
    }) {
        this.bump = fields.bump;
        this.whitelist = fields.whitelist;
        this.owner = fields.owner;
        this.payer = fields.payer;
        this.allowance = fields.allowance;
        this.amountBought = fields.amountBought;
    }

    static schema: Schema = {
        struct: {
            bump: "u8",
            whitelist: { array: { type: "u8", len: 32 } },
            owner: { array: { type: "u8", len: 32 } },
            payer: { array: { type: "u8", len: 32 } },
            allowance: "u64",
            amountBought: "u64",
        }
    };

    serialize(): Uint8Array {
        return serialize(TicketSchema.schema, this);
    }
}
