# hodlvoice
Hold an invoice for up to 24h. Accept/reject it at any time before expiration.

## Building
Currently hodlvoice depends on an unmerged pull request for cln-plugin: https://github.com/ElementsProject/lightning/pull/6091, so you need that and a folder structure that looks like this:
```
./lightning/plugins
./hodlvoice
```
then run `cargo build --release` in the hodlvoice folder with an up-to-date rust version: https://rustup.rs/. The plugin will be here: `./hodlvoice/target/release/hodlvoice`

## Installation
Build the plugin or get the binaries from the release page and 
put this in your lightning config:
```
plugin=/path/to/hodlvoice
```

## Documentation
### hodlvoice-add
`amount_msat label description [expiry] [fallbacks] [preimage] [exposeprivatechannels] [deschashonly]`

Create an invoice with the same parameters and return values as lightning-cli invoice, except cltv is hardcoded. Usage of -k is a must!
Basic example:
```
lightning-cli hodlvoice-add -k amount_msat=1000 label="bestpluginever" description=""
```

### hodlvoice-accept
`payment_hash`

Accept payment for a previously `hodlvoice-add`'ed invoice:
```
lightning-cli hodlvoice-accept 605079d1ab4514b3a2a2e0305c5c1beb37084572f5cbacb7cc519e05c7c48445
```

### hodlvoice-reject
`payment_hash`

Reject payment for a previously `hodlvoice-add`'ed invoice:
```
lightning-cli hodlvoice-reject 605079d1ab4514b3a2a2e0305c5c1beb37084572f5cbacb7cc519e05c7c48445
```

## Notes
There are some safety checks implemented to stop holding incoming htlcs if the invoice or htlcs are about to expire.

The invoice's payment_hash are saved to the cln database for persistency.
