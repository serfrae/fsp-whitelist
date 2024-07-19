import { PublicKey, TransactionInstruction, AccountMeta, SystemProgram } from "@solana/web3.js";
import { Numberu64, Numberi64 } from "./numbers";
import { getWhitelistAddress, getTicketAddress, toUnixTimestamp, getDuration } from "./utils";
import { PROGRAM_ID } from "./programId";
import { serialize, Schema } from "borsh";
import { getAssociatedTokenAddressSync, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";

export class InstructionBuilder {
    static createInstruction(accounts: AccountMeta[], data: Buffer): TransactionInstruction {
        return new TransactionInstruction({
            keys: accounts,
            programId: PROGRAM_ID,
            data,
        });
    }

    static initWhitelist(
        authority: PublicKey,
        mint: PublicKey,
        tokenProgram: PublicKey,
        instruction: InitWhitelist
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];

        const vault = getAssociatedTokenAddressSync(
            mint,
            whitelist, true,
            tokenProgram,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: true,
            },
            {
                pubkey: vault,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: tokenProgram,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
        ];

        const data = instruction.serialize();
        return InstructionBuilder.createInstruction(accounts, data);
    }

    static addUser(
        authority: PublicKey,
        mint: PublicKey,
        user: PublicKey,
        instruction: AddUser,
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];
        const ticket = getTicketAddress(user, whitelist)[0];

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: true,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: true,

            },
            {
                pubkey: user,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ticket,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
        ];

        let data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static removeUser(
        authority: PublicKey,
        mint: PublicKey,
        user: PublicKey,
        instruction: RemoveUser,
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];
        const ticket = getTicketAddress(user, whitelist)[0];

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: false,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: user,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ticket,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
        ];

        let data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static buyTokens(
        mint: PublicKey,
        user: PublicKey,
        tokenProgram: PublicKey,
        instruction: BuyTokens
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];
        const ticket = getTicketAddress(user, whitelist)[0];

        const vault = getAssociatedTokenAddressSync(
            mint,
            whitelist,
            true,
            tokenProgram,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const userTokenAccount = getAssociatedTokenAddressSync(
            mint,
            user,
            true,
            tokenProgram,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const ticketTokenAccount = getAssociatedTokenAddressSync(
            mint,
            ticket,
            true,
            tokenProgram,
            ASSOCIATED_TOKEN_PROGRAM_ID,
        );

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: vault,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: user,
                isSigner: true,
                isWritable: true,
            },
            {
                pubkey: ticket,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: ticketTokenAccount,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: userTokenAccount,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: tokenProgram,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
        ];

        let data = instruction.serialize();
        return InstructionBuilder.createInstruction(accounts, data);
    }

    static amendWhitelistSize(
        authority: PublicKey,
        mint: PublicKey,
        instruction: AmendWhitelist
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: true,
            },
        ];

        let data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static amendTimes(
        authority: PublicKey,
        mint: PublicKey,
        instruction: AmendTimes,
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: true,
            },
        ];

        let data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static allowRegistration(
        authority: PublicKey,
        mint: PublicKey,
        instruction: AllowRegistration,
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];

        let accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: true,
            },
        ];

        const data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static register(
        mint: PublicKey,
        user: PublicKey,
        instruction: Register,
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];
        const ticket = getTicketAddress(user, whitelist)[0];

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: user,
                isSigner: true,
                isWritable: true,
            },
            {
                pubkey: ticket,
                isSigner: false,
                isWritable: false,
            },
        ];

        let data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static unregister(
        authority: PublicKey,
        vault: PublicKey,
        mint: PublicKey,
        user: PublicKey,
        tokenProgram: PublicKey,
        instruction: Unregister,
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];
        const ticket = getTicketAddress(user, whitelist)[0];
        const ticketTokenAccount = getAssociatedTokenAddressSync(
            mint,
            ticket,
            true,
            tokenProgram,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: authority,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: vault,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: user,
                isSigner: true,
                isWritable: true,
            },
            {
                pubkey: ticket,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ticketTokenAccount,
                isSigner: false,
                isWritable: false,

            },
            {
                pubkey: tokenProgram,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
        ];

        let data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static depositTokens(
        mint: PublicKey,
        vault: PublicKey,
        depositor: PublicKey,
        tokenProgram: PublicKey,
        instruction: DepositTokens,
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];
        const depositorTokenAccount = getAssociatedTokenAddressSync(
            mint,
            depositor,
            true,
            tokenProgram,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        let accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: vault,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: depositor,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: depositorTokenAccount,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: tokenProgram,
                isSigner: false,
                isWritable: false,
            },
        ];

        let data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);

    }

    static startRegistration(
        authority: PublicKey,
        mint: PublicKey,
        instruction: StartRegistration
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: false,
            },
        ];

        const data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static startTokenSale(
        authority: PublicKey,
        mint: PublicKey,
        instruction: StartTokenSale
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: true,
            },
        ];

        const data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static transferTokens(
        authority: PublicKey,
        mint: PublicKey,
        user: PublicKey,
        tokenProgram: PublicKey,
        instruction: TransferTokens
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];
        const ticket = getTicketAddress(user, whitelist)[0];
        const vault = getAssociatedTokenAddressSync(
            mint,
            whitelist,
            true,
            tokenProgram,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const ticketTokenAccount = getAssociatedTokenAddressSync(
            mint,
            ticket,
            true,
            tokenProgram,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: true,
            },
            {
                pubkey: vault,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: user,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ticket,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ticketTokenAccount,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: tokenProgram,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
        ];

        const data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static withdrawTokens(
        authority: PublicKey,
        mint: PublicKey,
        recipient: PublicKey,
        tokenProgram: PublicKey,
        instruction: WithdrawTokens
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];
        const vault = getAssociatedTokenAddressSync(
            mint,
            whitelist,
            true,
            tokenProgram,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const recipientTokenAccount = getAssociatedTokenAddressSync(
            mint,
            recipient,
            true,
            tokenProgram,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: true,
            },
            {
                pubkey: vault,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: recipientTokenAccount,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: tokenProgram,
                isSigner: false,
                isWritable: false,
            },
        ];

        let data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static burnTicket(
        authority: PublicKey,
        mint: PublicKey,
        treasury: PublicKey,
        ticket: PublicKey,
        tokenProgram: PublicKey,
        instruction: BurnTicket,
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];
        const treasuryTokenAccount = getAssociatedTokenAddressSync(mint, treasury, true, tokenProgram, ASSOCIATED_TOKEN_PROGRAM_ID);
        const ticketTokenAccount = getAssociatedTokenAddressSync(mint, ticket, true, tokenProgram, ASSOCIATED_TOKEN_PROGRAM_ID);

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: true,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: treasury,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: treasuryTokenAccount,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: ticket,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: ticketTokenAccount,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: tokenProgram,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
                isSigner: false,
                isWritable: false,
            },
        ];

        const data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }

    static terminateWhitelist(
        authority: PublicKey,
        mint: PublicKey,
        recipient: PublicKey,
        tokenProgram: PublicKey,
        instruction: TerminateWhitelist,
    ): TransactionInstruction {
        const whitelist = getWhitelistAddress(mint)[0];
        const vault = getAssociatedTokenAddressSync(
            mint, 
            whitelist, 
            true, 
            tokenProgram, 
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const recipientTokenAccount = getAssociatedTokenAddressSync(
            mint, 
            recipient, 
            true, 
            tokenProgram, 
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const accounts = [
            {
                pubkey: whitelist,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: authority,
                isSigner: true,
                isWritable: true,
            },
            {
                pubkey: vault,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: mint,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: recipient,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: recipientTokenAccount,
                isSigner: false,
                isWritable: true,
            },
            {
                pubkey: tokenProgram,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
        ];

        const data = instruction.serialize();

        return InstructionBuilder.createInstruction(accounts, data);
    }
}

enum WhitelistInstruction {
    InitialiseWhitelist = 0,
    AddUser = 1,
    RemoveUser = 2,
    AmendWhitelistSize = 3,
    AmendTimes = 4,
    AllowRegister = 5,
    Register = 6,
    Unregister = 7,
    Buy = 8,
    DepositTokens = 9,
    StartRegistration = 10,
    StartTokenSale = 11,
    TransferTokens = 12,
    WithdrawTokens = 13,
    BurnTicket = 14,
    TerminateWhitelist = 15,

}

export class InitWhitelist {
    treasury: PublicKey;
    tokenPrice: Numberu64;
    whitelistSize: Numberu64;
    buyLimit: Numberu64;
    allowRegistration: boolean;
    registrationTimestamp: Numberi64;
    registrationDuration: Numberi64;
    saleTimestamp: Numberi64;
    saleDuration: Numberi64;

    constructor(fields: {
        treasury: PublicKey,
        tokenPrice: Numberu64,
        whitelistSize: Numberu64,
        buyLimit: Numberu64,
        allowRegistration: boolean,
        registrationStartTime: Date,
        registrationEndTime: Date,
        saleStartTime: Date,
        saleEndTime: Date,
    }) {
        WhitelistInstruction.InitialiseWhitelist;
        this.treasury = fields.treasury;
        this.tokenPrice = fields.tokenPrice;
        this.whitelistSize = fields.whitelistSize;
        this.buyLimit = fields.buyLimit;
        this.allowRegistration = fields.allowRegistration;
        this.registrationTimestamp = toUnixTimestamp(fields.registrationStartTime);
        this.registrationDuration = getDuration(
            toUnixTimestamp(fields.registrationStartTime),
            toUnixTimestamp(fields.registrationEndTime)
        );
        this.saleTimestamp = toUnixTimestamp(fields.saleStartTime);
        this.saleDuration = getDuration(
            toUnixTimestamp(fields.saleStartTime),
            toUnixTimestamp(fields.saleEndTime)
        );
    }

    static instructionType = WhitelistInstruction.InitialiseWhitelist;

    static schema: Schema = {
        struct: {
            mint: { array: { type: "u8", len: 32 } },
            treasury: { array: { type: "u8", len: 32 } },
            tokenPrice: "u64",
            buyLimit: "u64",
            whitelistSize: "u64",
            allowRegistration: "u8",
            registrationTimestamp: "i64",
            registrationDuration: "i64",
            saleTimestamp: "i64",
            saleDuration: "i64",
            tokenProgram: { array: { type: "u8", len: 32 } },
        }
    };

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1)
        instructionTypeBuffer.writeUint8(InitWhitelist.instructionType);
        return Buffer.concat([instructionTypeBuffer, serialize(InitWhitelist.schema, this)]);
    }
}

export class AddUser {
    static instructionType = WhitelistInstruction.AddUser;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(AddUser.instructionType);
        return instructionTypeBuffer;
    }
}

export class RemoveUser {
    static instructionType = WhitelistInstruction.RemoveUser;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(RemoveUser.instructionType);
        return instructionTypeBuffer;
    }
}

export class BuyTokens {
    amount: Numberu64;

    constructor(amount: Numberu64) {
        this.amount = amount;
    }

    static schema: Schema = {
        struct: {
            amount: "u64",
        }
    }

    static instructionType = WhitelistInstruction.Buy;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(BuyTokens.instructionType);
        return Buffer.concat([instructionTypeBuffer, serialize(BuyTokens.schema, this)]);
    }
}

export class AmendWhitelist {
    size: Numberu64;

    constructor(size: Numberu64) {
        this.size = size;
    }

    static schema: Schema = {
        struct: {
            size: "u64",
        }
    }

    static instructionType = WhitelistInstruction.AmendWhitelistSize;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(AmendWhitelist.instructionType);
        return Buffer.concat([instructionTypeBuffer, serialize(AmendWhitelist.schema, this)]);
    }
}

export class AmendTimes {
    registrationTimestamp?: Numberi64;
    registrationDuration?: Numberi64;
    saleTimestamp?: Numberi64;
    saleDuration?: Numberi64;

    constructor(fields: {
        registrationTimestamp?: Numberi64,
        registrationDuration?: Numberi64,
        saleTimestamp?: Numberi64,
        saleDuration?: Numberi64,
    }) {
        this.registrationTimestamp = fields.registrationTimestamp;
        this.registrationDuration = fields.registrationDuration;
        this.saleTimestamp = fields.saleTimestamp;
        this.saleDuration = fields.saleDuration;
    }

    static instructionType = WhitelistInstruction.AmendTimes;
}

export class AllowRegistration {
    allow: boolean;

    constructor(allow: boolean) {
        this.allow = allow;
    }

    static schema: Schema = {
        struct: {
            allowRegistration: "u8",
        }
    }

    static instructionType = WhitelistInstruction.AllowRegister;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(AllowRegistration.instructionType);
        return Buffer.concat(
            [instructionTypeBuffer, serialize(AllowRegistration.schema, this)]
        );
    }
}

export class Register {
    static instructionType = WhitelistInstruction.Register;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(Register.instructionType);
        return instructionTypeBuffer;
    }
}

export class Unregister {
    static instructionType = WhitelistInstruction.Unregister;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(Unregister.instructionType);
        return instructionTypeBuffer;
    }
}

export class DepositTokens {
    amount: Numberu64;

    constructor(amount: Numberu64) {
        this.amount = amount;
    }

    static schema: Schema = {
        struct: {
            amount: "u64",
        }
    }

    static instructionType = WhitelistInstruction.DepositTokens;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(DepositTokens.instructionType);
        return instructionTypeBuffer;
    }
}

export class StartRegistration {
    static instructionType = WhitelistInstruction.StartRegistration;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(StartRegistration.instructionType);
        return instructionTypeBuffer;
    }
}

export class StartTokenSale {
    static instructionType = WhitelistInstruction.StartTokenSale;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(StartTokenSale.instructionType);
        return instructionTypeBuffer;
    }
}

export class TransferTokens {
    amount: Numberu64;

    constructor(amount: Numberu64) {
        this.amount = amount;
    }

    static schema: Schema = {
        struct: {
            amount: "u64",
        }
    }

    static instructionType = WhitelistInstruction.TransferTokens;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(TransferTokens.instructionType);
        return instructionTypeBuffer;
    }
}

export class WithdrawTokens {
    amount: Numberu64;

    constructor(amount: Numberu64) {
        this.amount = amount;
    }

    static schema: Schema = {
        struct: {
            amount: "u64",
        }
    }

    static instructionType = WhitelistInstruction.WithdrawTokens;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(WithdrawTokens.instructionType);
        return instructionTypeBuffer;
    }
}

export class BurnTicket {
    static instructionType = WhitelistInstruction.BurnTicket;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(BurnTicket.instructionType);
        return instructionTypeBuffer;
    }
}

export class TerminateWhitelist {
    static instructionType = WhitelistInstruction.TerminateWhitelist;

    serialize(): Buffer {
        const instructionTypeBuffer = Buffer.alloc(1);
        instructionTypeBuffer.writeUint8(TerminateWhitelist.instructionType);
        return instructionTypeBuffer;
    }
}
