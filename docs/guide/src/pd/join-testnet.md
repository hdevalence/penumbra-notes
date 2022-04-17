# Joining a Testnet

Currently we only provide instructions for running a fullnode deployment. A
fullnode will sync with the network but will not have any voting power, and will
not be eligible for staking or funding stream rewards. For more information on
what a fullnode is, see the [Tendermint
documentation](https://docs.tendermint.com/v0.35/nodes/#full-node).

## Configuring your node

Run the following:

```console
$ tendermint init full
```

and you will have the Tendermint files created necessary for your fullnode:

```console
$ ls $HOME/.tendermint
config	data

$ ls $HOME/.tendermint/config/
config.toml	genesis.json	node_key.json
```

Next you'll need to modify the persistent peers list to specify our primary testnet node as a permanent peer for your node. Open `$HOME/.tendermint/config/config.toml` and find the `persistent-peers` line and make it look like so:

```toml
persistent-peers = "TODO_IDENTIFIER_GOES_HERE@192.167.10.21:26656"
```

Then you'll need to download our genesis JSON file and replace the automatically generated one.
You can find the genesis JSON files for all of our testnets [in our repository](https://github.com/penumbra-zone/penumbra/tree/main/testnets).

The testnet directories are numbered in increasing order, find and download the latest JSON file and copy it to `$HOME/.tendermint/config/genesis.json`.

## Starting your node

After configuration is complete, you're ready to start your node.

First, start the `pd` binary:

```console
$ export RUST_LOG="warn,pd=debug,penumbra=debug,jmt=info" # or some other logging level
$ cargo run --release --bin pd start --rocks-path $HOME/.rocksdb 
```

Then (perhaps in another terminal) start Tendermint:

```console
$ tendermint start
```

You should now be participating in the network!
