# SLP Hash Timelock Contract

Nomenclature and (modified) script taken from https://github.com/bitcoin/bips/blob/master/bip-0199.mediawiki.

## Usage

### Setup

Note: Apart from the RPC configuration, the below commands for the setup can also be done in the GUI. They are done via the commandline for reproducability here.

1. Install Electron Cash SLP Edition (https://simpleledger.cash/project/electron-cash-slp-edition/) and locate its executable (assumed to be `./electron-cash` from here on).
2. Run `./electron-cash --testnet setconfig rpcport 7777` to set the JSON RPC port.
3. Run these commands and keep the username and password ready:
    - `./electron-cash --testnet getconfig rpcuser`
    - `./electron-cash --testnet getconfig rpcpassword`
4. Create the Seller wallet (seller is the party that will receive the SLP tokens, and presumably sell some other cryptoasset of value, e.g. ETH or BTC):
    - `./electron-cash --testnet create_slp -w ./sellerwallet`
5. Create the Buyer wallet (buyer is the party that sends the SLP tokens, and presumably buy some other cryptoasset of value, e.g. ETH or BTC):
    - `./electron-cash --testnet create_slp -w ./buyerwallet`
6. Run Electron Cash SLP edition either as daemon or GUI:
    - Either: `./electron-cash --testnet` (GUI, for testing)
    - Or: `./electron-cash --testnet daemon` (no GUI, for servers)
7. In a new terminal, run `./electron-cash --testnet daemon load_wallet -w ./sellerwallet` to switch to Seller's wallet.
8. Get the address from seller's wallet:
    - Run `curl --data-binary '{"id":0,"method":"getunusedaddress"}' http://<rpcuser>:<rpcpassword>@127.0.0.1:7777`, where `rpcuser` and `rpcpassword` are the values from above, and keep the generated address ready.
    - Also send some tBCH to that address for the 'gas' for txs. 0.00010000 tBCH suffices. You might need to convert the address to bchtest for this.
    - Run `curl --data-binary '{"id":0,"method":"slp_add_token","params":{"token_id":"<token_id>"}}' http://<rpcuser>:<rpcpassword>@127.0.0.1:7777`, where `token_id` is the token you want to send.
9. Run `./electron-cash --testnet daemon load_wallet -w ./buyerwallet` to switch to Buyer's wallet.
10. Fund the buyer's wallet:
    - Run `curl --data-binary '{"id":0,"method":"getunusedaddress"}' http://<rpcuser>:<rpcpassword>@127.0.0.1:7777`.
    - Send some SLP tokens to that address (prefix is "slptest"; if prefix is "bchtest", you loaded a non-SLP wallet previously; restart Electron Cash SLP in this case and load the daemon wallet anew).
    - Also send some tBCH to that address for the 'gas' for txs. 0.00010000 tBCH suffices. You might need to convert the address to bchtest for this.
    - Run `curl --data-binary '{"id":0,"method":"slp_add_token","params":{"token_id":"<token_id>"}}' http://<rpcuser>:<rpcpassword>@127.0.0.1:7777`, where `token_id` is the token you've sent.
11. Setup complete!

### Fund HTLC
From the setup above, you should have these values ready:
- The JSON RPC URI `http://<rpcuser>:<rpcpassword>@127.0.0.1:7777`
- An unused address from Seller.

Also, you should be switched to Buyer's wallet.

Now we can fund the HTLC:

1. Clone this repository (`git clone https://github.com/EyeOfPython/slp-htlc.git`)
2. Run `cargo run -- gen-secret` and keep the secret and secret hash ready (this would happen on Sellers's computer).
   Example output:
   ```
   secret: eb5078d1f306784715040d1846871adfb476e848e7e7c6aaec1822bce35311dd
   secret hash: 6af9c9b8635b453c9ce522bf44a11f0afcd8ad9d
   ```
3. Run the following command:
    ```bash
    cargo run -- \
        send-htlc \
        --token-id <token-id> \
        --amount <amount> \
        --seller-address <seller-address> \
        --secret-hash <secret-hash> \
        --timeout <timeout> \
        --uri <uri>
    ```
    Where:
    - token-id: token you've sent
    - amount: amount of the token you want to lock into the HTLC
    - seller-address: address from Seller we've previously generated
    - secret-hash: the secret hash Seller has provided us for this setup (from `gen-secret`)
    - timeout: UNIX timestamp for when this HTLC expires
    - uri: JSON RPC URI 
    Example:
    ```
    $ cargo run -- \
        send-htlc \
        --token-id bb309e48930671582bea508f9a1d9b491e49b69be3d6f372dc08da2ac6e90eb7 \
        --amount 1 \
        --seller-address slptest:qrzurumzwn7kwtcszk3jgpgfgecp4ws8wcvvxgnrts \
        --secret-hash 6af9c9b8635b453c9ce522bf44a11f0afcd8ad9d \
        --timeout 1607333086 \
        --uri http://<rpcuser>:<rpcpassword>@127.0.0.1:7777
    buyer address: slptest:qqcjtkw3a3mdh26y0ryrtfmxf4y2jhle6y72nalmlq
    timeout: 1607333086
    contract UTXO: 6912c3a61f715dba3067e0a17e5613f9d19edeea593b9456f952bd34de06faa5:1
    ```
4. Keep keep the buyer address, timeout and contract UTXO handy (this would be sent to Seller).
5. HTLC funded!

### Redeem HTLC

From the fund step above, you should have these values ready:
- The JSON RPC URI `http://<rpcuser>:<rpcpassword>@127.0.0.1:7777`
- The seller address from the HTLC.
- The buyer address from the HTLC.
- The timeout for the HTLC.
- The contract UTXO for the HTLC.
- The secret (preimage) for the HTLC.

1. Run `./electron-cash --testnet daemon load_wallet -w ./sellerwallet` to switch to Seller's wallet.
2. Run the following command:
   ```
    $ cargo run -- \
        redeem-htlc \
        --contract-utxo <contract-utxo> \
        --buyer-address <buyer-address> \
        --secret <secret> \
        --timeout <timeout> \
        --seller-address <seller-address> \
        --uri <uri>
   ```
   Example:
   ```
   $ cargo run -- \
        redeem-htlc \
        --contract-utxo 6912c3a61f715dba3067e0a17e5613f9d19edeea593b9456f952bd34de06faa5:1 \
        --buyer-address slptest:qqcjtkw3a3mdh26y0ryrtfmxf4y2jhle6y72nalmlq \
        --secret eb5078d1f306784715040d1846871adfb476e848e7e7c6aaec1822bce35311dd \
        --timeout 1607333086 \
        --seller-address slptest:qrzurumzwn7kwtcszk3jgpgfgecp4ws8wcvvxgnrts \
        --uri http://<rpcuser>:<rpcpassword>@127.0.0.1:7777
   contract_amount: 10000
   token_id: TokenId(Sha256d(bb309e48930671582bea508f9a1d9b491e49b69be3d6f372dc08da2ac6e90eb7))
   57d3446c56b3557825cbb8b7f618d0ccef0fd26bef217e30d838ec413dcd2d86
   ```
3. HTLC redeemed!

# Timeout HTLC
Run the Fund HTLC section again to generate a new HTLC.

This will give you these values:
- The JSON RPC URI `http://<rpcuser>:<rpcpassword>@127.0.0.1:7777`
- The seller address from the HTLC.
- The buyer address from the HTLC.
- The timeout for the HTLC.
- The contract UTXO for the HTLC.
- The secret hash for the HTLC.

Example:

```
$ cargo run -- gen-secret
secret: e51e578fd319d8a1b8b55256b3a0c9773a931252398f13847b89d789b9835e00
secret hash: 82fa07e4ee949640eb4eb5ed509c8a8732640b97
$ ./electron-cash --testnet daemon load_wallet -w ./buyerwallet
$ cargo run -- \
    send-htlc \
    --token-id bb309e48930671582bea508f9a1d9b491e49b69be3d6f372dc08da2ac6e90eb7 \
    --amount 1 \
    --seller-address slptest:qq49s69jttj8ay9fuqknttrwxesztm88vqfrk4c040 \
    --secret-hash 82fa07e4ee949640eb4eb5ed509c8a8732640b97 \
    --timeout 1607334641 \
    --uri http://<rpcuser>:<rpcpassword>@127.0.0.1:7777
buyer address: slptest:qqcjtkw3a3mdh26y0ryrtfmxf4y2jhle6y72nalmlq
timeout: 1607334641
contract UTXO: ef44a5ee9e481b8eb2343d8e46417281a90a02051f3bc1ef901229fcab5b555f:1
```

1. Wait for MTP timeout to arrive.
2. Run the following command:
    ```
    $ cargo run -- \
        timeout-htlc \
        --contract-utxo <contract-utxo>
        --seller-address <seller-address>
        --secret-hash <secret-hash>
        --timeout <timeout>
        --buyer-address <buyer-address>
        --uri <uri>
    ```
    Example:
    ```
    $ cargo run -- \
        timeout-htlc \
        --contract-utxo ef44a5ee9e481b8eb2343d8e46417281a90a02051f3bc1ef901229fcab5b555f:1 \
        --seller-address slptest:qq49s69jttj8ay9fuqknttrwxesztm88vqfrk4c040 \
        --secret-hash 82fa07e4ee949640eb4eb5ed509c8a8732640b97 \
        --timeout 1607334641 \
        --buyer-address slptest:qqcjtkw3a3mdh26y0ryrtfmxf4y2jhle6y72nalmlq \
        --uri http://<rpcuser>:<rpcpassword>@127.0.0.1:7777
    contract_amount: 10000
    token_id: TokenId(Sha256d(bb309e48930671582bea508f9a1d9b491e49b69be3d6f372dc08da2ac6e90eb7))
    dff9d9964d5276794d82f5e930aeb9f3a2088dd34744a6d815e89e19d6fd4203
    ```
3. HTLC refunded!
