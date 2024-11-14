# timet-tui

Manage Timet (partially) from your terminal.

Features:
- [x] overview of year
- [x] overview of month
- [ ] details of week
- [ ] details of project
- [ ] CLI Mode for simple tasks

## Configuring

There are two options for configuring this application. The active locations will contain the configuration and the database. The database 
is not important and can be generated within seconds by pressing `r` when the application is open.

The configuration locations in order of precedence:
1. `TIMET_CONFIG_HOME` set to any folder containing `timet.toml`
2. `XDG_CONFIG_HOME` is set, it will read from  `$XDG_CONFIG_HOME/timet.toml`.

Create the API key when logged into *Timet*, and store it in the environment variable `TIMET_API_KEY`.
Storing this key securely is (not yet) in the scope for this application. **Do not store the API key in configuration files!**

### Default configuration

There is no automatic generation of `config.toml`, create it like this (endpoint can be found on slack)
Endpoint can be defined with or without https, but http will fail.

```toml
[api]
endpoint = '****'
```

### Running

After completing configuration and setting up the environment variables, the application is started 
either by setting `TIMET_API_KEY` and running it, or by prefixing the run command with
`TIMET_API_KEY=abcdef1234567 ./timet-tui`.

## Development

This application bundles its own version of SQLite. This also means it supports the compile
flags from SQLite itself. These can be found [here](https://www.sqlite.org/compile.html)

A local DB named `timet.db` will automatically be initialised and tables created.
The environment variables above will be used for development so make sure to set these.
