# Assignment

I've omitted the assignment (recruiter told me to).

# Interpretation of Assignment

I've interpret the assignment such that you care about the amount of gas used
for a particular Ethereum transaction. Not the amount of Eth paid in fees. So my
solution is gas price agnostic, meaning it is does not use any [EIP-1559][eip15519]
fields - `max_priority_fee_per_gas` / `max_fee_per_gas` - as those are not relevant
for gas usage estimation.

Furthermore I've interpreted the assignemnt such that I'm allowed to use [
RPC method `eth_estimateGas` with the Alchemy API][alchemy] and that I'm allowed to
use [`revm`][revm] for estimates too.

# Setup

To setup development see [SETUP.md](SETUP.md)

# Design

I've split the solution into three crates:

-   `gastimate` - the binary (tiny crate)
-   `gastimator-rest` - a REST server (small crate)
-   `gastimator` - a library, with the logic and models (medium sized crate)

## CLI

`gastimate` uses [`clap`][clap] to start the server in `gastimator-rest` using `async fn run(config: &Config)`.

## REST Server

`gastimator-rest` uses [`axum`][axum] to spin up a REST server, and uses `gastimator`.

## Logic

`gastimator` has two key components:

-   **Local** transaction simulation with gas estimation using [`revm`][revm]
-   **Remote** gas estimation using [Alchemy's RPC method `eth_estimateGas`][alchemy]

I'm using [`reqwest`][reqwest] to build a small RPC client consuming the Alchemy API.

If both a _local_ and _remote_ estimate was successfully obtained, the max value of the two
is returned as the estimate.

### Request

You can send requests to this software, the `gastimate` binary, using two different
formats. Either send a `Transaction` objects, or send a `RawTransaction` with an
[`rlp` (Recursive Length Prefix, a binary encoding)][rlp] hex string, see below:

#### Transaction Request Model

```rust
struct Transaction {
    nonce: Option<u64>,
    from: Option<Address>,
    to: TxKind,
    value: U256,
    gas_limit: Option<Gas>,
    input: Bytes,
}
```

#### `rlp`

You can find the RLP [by navigating to a TX on `Etherscan`][etherscan] and then clicking
the list button on the right hand side and then `"Get Raw Tx Hex"`, [see screenshot][.github/etherscan_get_rlp.png]. The pass the value into;

```rust
struct RawTransaction {
    rlp: Bytes
}
```

### `gas_limit`

`gas_limit` you typically don't need to or ought to specify. The purpose of this
software is to provide you with an estimate.

If you do provide a value, and if it is too low, e.g. `10` for a simple ETH transfer,
which requires `21,000` gas, `gastimator` will return an error:
`GasExceedsLimit { estimated_cost: Some(Gas(21000)), gas_limit: Gas(10) }`

### Caching

If **both** `nonce` and `from` is set I will try to read a previous gas estimate from
a cache, since same value of `(nonce, from)` tuple ought to mean it is the same
transaction.

I use a newtype around [`dashmap`][dashmap] for cache, which is fast concurrent map in Rust,
essentially a drop-in-replacement for `RwLock<HashMap<_, _>>`.

But I use the `Transaction` in **its entirety** as a cache key, meaning
if you for example send a similar transaction but other value of `gas_limit` it will
be a cache miss. I do not cache transaction which lacks either `nonce` or `from`.

# Requirements

> [!IMPORTANT]
> You MUST have an Alchemy API key, either [create an account](https://auth.alchemy.com/signup) (there is a FREE plan)
> or ask Alex Cyon for his.
> Either export the key as an environment variable named `ALCHEMY_API_KEY` (see [SETUP](SETUP.md))
> or pass it as an argument like so `cargo run --release --locked -- --key <KEY_HERE>`

## Make commands

If you have exported `ALCHEMY_API_KEY` (either direclly in your shell or inside an `.envrc.secret` (gitignored)) you can use these `make` commands.

### Run

```sh
make run
```

and then in another shell, you can send a request to the locally running server, typically (using `rlp`):

```sh
curl http://0.0.0.0:3000/rlp -X POST \
  -H "Content-Type: application/json" \
  -d '{"rlp": "02f902db01820168841dcd6500843d831e6783027a6d9466a9893cc07d91d95644aedd05d03f95e1dba8af8803bbae1324948000b9026424856bc30000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000020b080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000003bbae1324948000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000096cdea52111684fd74ec6cdf31dd97f395737a5d00000000000000000000000000000000000000000000000003bbae132494800000000000000000000000000000000000000000000000001e28ba62f4e8c66e7b00000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000000aff507ac29b8cea2fb10d2ad14408c2d79a35adc001a0ff907e592e412943d4f7136aeb55f9fcf701ef295c7cd620be07ffe037de5b58a05889a72e62156c986ccc657bcdba1f91c481e817ee607408320d312416fa3a67"}'
```

And you should see something like:

```sh
{"gas_usage":{"AtLeastWithEstimate":{"kind":{"ContractCall":{"with_native_token_transfer":true}},"at_least":2600,"estimate":147649}},"was_last_response":true,"time_elapsed_in_millis":596}
```

where `147649` is the estimated gas usage.

Or alternatively if you wanna pass the transaction using individual JSON fields:

```sh
curl http://0.0.0.0:3000/tx -X POST \
  -H "Content-Type: application/json" \
  -d '{
  "nonce": 360,
  "to": "0x66a9893cc07d91d95644aedd05d03f95e1dba8af",
  "value": "0x3bbae1324948000",
  "input": "0x24856bc30000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000020b080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000003bbae1324948000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000096cdea52111684fd74ec6cdf31dd97f395737a5d00000000000000000000000000000000000000000000000003bbae132494800000000000000000000000000000000000000000000000001e28ba62f4e8c66e7b00000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000000aff507ac29b8cea2fb10d2ad14408c2d79a35ad"
}'
```

Or a simple ETH transfer (and `value` as decimal):

```sh
curl http://0.0.0.0:3000/tx -X POST \
  -H "Content-Type: application/json" \
  -d '{
  "to": "0x11a9893cc07d91d95644aedd05d03f95e1dbaccd",
  "value": "123456789"
}'
```

> [!NOTE]
> Note the different endpoints `/rlp` vs `/tx`
> Also make sure you use the correct port if you specified another
> port using `--port` flag.

> [!NOTE]
> Note that `value` is in wei, not in full Eth.

#### Gas limit

If you set a too low gas limit on your transaction, e.g something less than `21_000` for
a simple ETH transfer:

```sh
$ curl http://0.0.0.0:3000/tx -X POST \
  -H "Content-Type: application/json" \
  -d '{
  "to": "0x11a9893cc07d91d95644aedd05d03f95e1dbaccd",
  "value": "123456789", "gas_limit": 1
}'
```

then error `GasExceedsLimit` is returned:

```
GasExceedsLimit { estimated_cost: Some(Gas(21000)), gas_limit: Gas(1) }
```

### Test

You _MUST_ export `ALCHEMY_API_KEY` variable to run **integration** tests:

```sh
make itest
```

You can run unit tests without `ALCHEMY_API_KEY`:

```sh
make utest
```

## Run by passing `ALCHEMY_API_KEY` as argument

```sh
cargo run --release --locked -- --key <ALCHEMY_API_KEY>
```

# Help

See help, for which arguments to pass:

```sh
make help
```

[clap]: https://crates.io/crates/clap
[axum]: https://crates.io/crates/axum
[revm]: https://crates.io/crates/revm
[reqwest]: https://crates.io/crates/reqwest
[alchemy]: https://docs.alchemy.com/reference/eth-estimategas
[etherscan]: https://etherscan.io/tx/0x6e9710bc55d7498934c22e9accad4c11810f6e86f51e1d6def3d750026cae1ab
[rlp]: https://ethereum.org/en/developers/docs/data-structures-and-encoding/rlp/
[dashmap]: https://crates.io/crates/dashmap
[eip15519]: https://eips.ethereum.org/EIPS/eip-1559
