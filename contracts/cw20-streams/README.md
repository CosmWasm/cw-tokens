# CW20 Streams

This contract enables the creation of cw20 token streams, which allows a cw20 payment to be vested continuously over time. This contract must be instantiated with a cw20 token address, after which any number of payment streams can be created from a single contract instance.

## Instantiation

To instantiate a new instance of this contract you must specify a contract owner, and the cw20 token address used for the streams. Only one cw20 token can be used for payments for each contract instance.

## Creating a Stream
A stream can be created using the cw20 [Send / Receive](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/README.md#receiver) flow. This involves triggering a Send message from the cw20 token contract, with a Receive callback that's sent to the token streaming contract. The callback message must include the start time and end time of the stream in seconds, as well as the payment recipient. 

## Withdrawing payments
Streamed payments can be claimed continously at any point after the start time by triggering a Withdraw message.

## Development
### Compiling

To generate a development build run:
```
cargo build
```

To generate an optimized build run:

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.3
```

### Testing
To execute unit tests run:
```
cargo test
```

### Lint
To lint repo run:
```
cargo fmt
```

