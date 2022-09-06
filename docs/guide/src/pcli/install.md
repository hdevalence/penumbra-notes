# Installing `pcli`

### Installing the Rust toolchain

This requires that you install a recent stable version
of the Rust compiler, installation instructions for which you can find
[here](https://www.rust-lang.org/learn/get-started). Don't forget to reload your shell so that
`cargo` is available in your `\$PATH`!

### Installing build prerequisites

#### Linux

You may need to install some additional packages in order to build `pcli`,
depending on your distribution. For a bare-bones Ubuntu installation, you can
run:

```bash
sudo apt-get install build-essential pkg-config libssl-dev clang
```

#### macOS

You may need to install the command-line developer tools if you have never done
so:

```bash
xcode-select --install
```

### Cloning the repository

Once you have installed the above tools, you can clone the repository:

```bash
git clone https://github.com/penumbra-zone/penumbra
```

To build the version of `pcli` compatible with the current testnet, navigate to the penumbra folder, fetch the latest from the repository, and check out the latest tag for the current [testnet](https://github.com/penumbra-zone/penumbra/releases):

```bash
cd penumbra && git fetch && git checkout 028-harpalyke && cargo update
```

### Building the `pcli` client software

Then, build the `pcli` tool using `cargo`:

```bash
cargo build --release --bin pcli
```

Because you are building a work-in-progress version of the client, you may see compilation warnings,
which you can safely ignore.
