import { PublicKey } from "@solana/web3.js";
import { Numberi64 } from "./numbers";
import { PROGRAM_ID } from "./programId";

export function getWhitelistAddress(mint: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync([mint.toBuffer()], PROGRAM_ID);
}

export function getTicketAddress(user: PublicKey, whitelist: PublicKey): [PublicKey, number] {
    return PublicKey.findProgramAddressSync([user.toBuffer(), whitelist.toBuffer()], PROGRAM_ID);
}

export function toUnixTimestamp(date: Date): Numberi64 {
    return new Numberi64(Math.floor(date.getTime()) / 1000);

}

export function getDuration(start: Numberi64, end: Numberi64): Numberi64 {
    return end.sub(start);
}
