## A Minimal Contract that implements the [Cw20 Receiver Interface](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/README.md#receiver)

This is a minimal example of a CosmWasm contract that implements the [Cw20 Receiver Interface](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/README.md#receiver)

---

**NOTE - This contract is only meant to be an example of how the cw20 receiver interface can be implemented, it is by no means production ready**

This contract accept will accept a receive message from **any** smart contract, it has no logic to verify that the sending contract correctly implements the Cw20 spec.

**You will most likely want to implement some additional custom logic (for example a whitelist) to verify that the messages are coming from a smart contract you want to accept messages from**

There is a simple example included to show how a whitelist could be implemented