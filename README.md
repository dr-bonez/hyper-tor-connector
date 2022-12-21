# hyper-tor-connector

Exports a connector for hyper that allows you to make tor connections.

## Features

### socks (default)

Connects over socks5 to a tor daemon to make tor connections.

### arti

Uses [arti-client](https://crates.io/arti-client) to make tor connections.

Please note that onion services will not be supported until arti 1.2.0.
