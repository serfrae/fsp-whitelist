#!/bin/sh
spl-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb create-token ./mint.json
spl-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb create-account GJieAPVSo9kpoKtPobCiD7KK94KfrDixTwSqzy64B5vQ 
spl-token --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb mint GJieAPVSo9kpoKtPobCiD7KK94KfrDixTwSqzy64B5vQ 100000000 ELNuFkcenyub4x14VYLQjRskgXaWZbWdZxAyDCr4gcGs 
