import BN from "bn.js";
import { Buffer } from "buffer";

// u64 Typescript
export class Numberu64 extends BN {
    static MAX_VALUE = new BN('18446744073709551615'); // 2^64 - 1

    //Construct a Numberu64 from number, string, or Buffer
    constructor(value: number | string | number[] | Uint8Array | Buffer | BN) {
        super(value, 10);
        if (this.isNeg() || this.gt(Numberu64.MAX_VALUE)) {
            throw new Error('Numberu64 must be between 0 and 2^64-1');
        }
    }

    toBuffer(): Buffer {
        return this.toArrayLike(Buffer, 'le', 8);
    }

    static fromBuffer(buffer: Buffer): Numberu64 {
        if (buffer.length !== 8) {
            throw new Error('Buffer must be 8 bytes for Numberu64');
        }
        return new Numberu64(new BN(buffer, 'le'));
    }
}

// i64 Typescript
export class Numberi64 extends BN {
    static MIN_VALUE = new BN('-9223372036854775808'); // -2^63
    static MAX_VALUE = new BN('9223372036854775807');  // 2^63 - 1

    //Construct a Numberi64 from number, string, or Buffer
    constructor(value: number | string | number[] | Uint8Array | Buffer | BN) {
        super(value, 10);
        if (this.lt(Numberi64.MIN_VALUE) || this.gt(Numberi64.MAX_VALUE)) {
            throw new Error('Numberi64 must be between -2^63 and 2^63-1');
        }
    }

    toBuffer(): Buffer {
        return this.toTwos(64).toArrayLike(Buffer, 'le', 8);
    }

    static fromBuffer(buffer: Buffer): Numberi64 {
        if (buffer.length !== 8) {
            throw new Error('Buffer must be 8 bytes for Numberi64');
        }
        const bn = new BN(buffer, 'le');
        return new Numberi64(bn.fromTwos(64));
    }
}

